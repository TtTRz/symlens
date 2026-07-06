#[cfg(feature = "mcp")]
pub mod server {
    use crate::commands::IndexProvider;
    use crate::index::{indexer, storage};
    use crate::model::project::RootInfo;
    use crate::model::symbol::SymbolKind;
    use crate::output::json as fmt;
    use parking_lot::RwLock;
    use rmcp::ServiceExt;
    use rmcp::handler::server::ServerHandler;
    use rmcp::handler::server::wrapper::Parameters;
    use rmcp::model::ErrorData as McpError;
    use rmcp::tool;
    use rmcp::tool_handler;
    use rmcp::tool_router;
    use schemars::JsonSchema;
    use serde::Deserialize;
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Arc, LazyLock};

    // ─── Static index cache ──────────────────────────────────────────

    static INDEX_CACHE: LazyLock<RwLock<HashMap<String, Arc<IndexProvider>>>> =
        LazyLock::new(|| RwLock::new(HashMap::new()));

    /// Auto-detect workspace: if root has symlens.workspace.toml, use workspace mode.
    fn load_provider(root: &std::path::Path) -> Option<Arc<IndexProvider>> {
        // Try single-root cache key first
        let input_key = cache_key(root);
        if let Some(idx) = INDEX_CACHE.read().get(&input_key) {
            return Some(Arc::clone(idx));
        }

        // Auto-detect workspace from symlens.workspace.toml
        let workspace = crate::config::WorkspaceConfig::load(root).is_some();

        if let Ok(provider) = IndexProvider::load(Some(root.to_string_lossy().as_ref()), workspace)
        {
            let arc = Arc::new(provider);
            // Use provider's own hash as cache key (stable across path variants)
            let stable_key = arc.socket_hash();
            {
                let mut cache = INDEX_CACHE.write();
                cache.insert(stable_key, Arc::clone(&arc));
                cache.insert(input_key, Arc::clone(&arc));
            }
            Some(arc)
        } else {
            None
        }
    }

    fn invalidate_key(key: &str) {
        INDEX_CACHE.write().remove(key);
    }

    fn cache_key(root: &std::path::Path) -> String {
        blake3::hash(root.to_string_lossy().as_bytes()).to_hex()[..16].to_string()
    }

    fn mcp_error(msg: impl std::fmt::Display) -> McpError {
        McpError::internal_error(msg.to_string(), None)
    }

    // ─── Parameter structs ───────────────────────────────────────────

    #[derive(Debug, Deserialize, JsonSchema)]
    pub struct IndexParams {
        /// Project root path
        pub path: String,
        /// Force re-index even if cache exists
        #[serde(default)]
        pub force: Option<bool>,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    pub struct IndexWorkspaceParams {
        /// List of project root paths for workspace mode
        pub roots: Vec<String>,
        /// Force re-index even if cache exists
        #[serde(default)]
        pub force: Option<bool>,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    pub struct SearchParams {
        /// Project root path
        pub path: String,
        /// Search query
        pub query: String,
        /// Max results (default 10)
        #[serde(default)]
        pub limit: Option<usize>,
        /// Filter by symbol kind (e.g. "function", "struct", "method")
        #[serde(default)]
        pub kind: Option<String>,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    pub struct SymbolParams {
        /// Project root path
        pub path: String,
        /// Symbol ID (format: "relative/path.rs::QualifiedName#kind")
        pub symbol_id: String,
        /// Include source code in the result
        #[serde(default)]
        pub source: Option<bool>,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    pub struct OutlineParams {
        /// Project root path
        pub path: String,
        /// File path (omit for project-wide outline)
        #[serde(default)]
        pub file: Option<String>,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    pub struct RefsParams {
        /// Project root path
        pub path: String,
        /// Symbol name to find references for
        pub name: String,
        /// Filter by reference kind (e.g. "call", "type", "import", "field", "constructor")
        #[serde(default)]
        pub kind: Option<String>,
        /// Max results (default 50)
        #[serde(default)]
        pub limit: Option<usize>,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    pub struct ImpactParams {
        /// Project root path
        pub path: String,
        /// Symbol name to analyze impact for
        pub name: String,
        /// Max traversal depth (default 3)
        #[serde(default)]
        pub depth: Option<usize>,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    pub struct CallersParams {
        /// Project root path
        pub path: String,
        /// Symbol name to find callers for
        pub name: String,
        /// Max results (default 20)
        #[serde(default)]
        pub limit: Option<usize>,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    pub struct CalleesParams {
        /// Project root path
        pub path: String,
        /// Symbol name to find callees for
        pub name: String,
        /// Max results (default 20)
        #[serde(default)]
        pub limit: Option<usize>,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    pub struct LinesParams {
        /// Project root path
        pub path: String,
        /// File path relative to project root
        pub file: String,
        /// Start line (1-based, inclusive)
        pub start: u32,
        /// End line (1-based, inclusive)
        pub end: u32,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    pub struct DiffParams {
        /// Project root path
        pub path: String,
        /// Git ref to diff from (e.g. "HEAD~1", "main")
        pub from: String,
        /// Git ref to diff to (e.g. "HEAD")
        pub to: String,
        /// Filter by symbol kind (e.g. "function", "struct")
        #[serde(default)]
        pub kind: Option<String>,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    pub struct StatsParams {
        /// Project root path
        pub path: String,
    }

    // ─── Server struct + tool methods ────────────────────────────────

    #[derive(Clone)]
    pub struct SymLensMcp;

    #[tool_router]
    impl SymLensMcp {
        #[tool(
            description = "Index a project directory with tree-sitter. Returns symbol count and timing."
        )]
        fn symlens_index(
            &self,
            Parameters(params): Parameters<IndexParams>,
        ) -> Result<String, McpError> {
            let root = PathBuf::from(&params.path);
            let force = params.force.unwrap_or(false);

            let prev_index = if force {
                None
            } else {
                storage::load(&root).ok().flatten()
            };

            match indexer::index_project_incremental(
                &root,
                100_000,
                prev_index.as_ref(),
                &indexer::WalkOptions::default(),
            ) {
                Ok(result) => match storage::save(&result.index) {
                    Ok(cache_path) => {
                        let key = cache_key(&root);
                        invalidate_key(&key);
                        invalidate_key(&result.index.root_hash);
                        Ok(serde_json::to_string_pretty(&json!({
                            "files": result.index.file_symbols.len(),
                            "symbols": result.index.symbols.len(),
                            "duration_ms": result.duration_ms,
                            "cache": cache_path.to_string_lossy(),
                        }))
                        .unwrap_or_default())
                    }
                    Err(e) => Err(mcp_error(format!("Save failed: {e}"))),
                },
                Err(e) => Err(mcp_error(format!("Index failed: {e}"))),
            }
        }

        #[tool(
            description = "Index a workspace with multiple project roots. Enables cross-project symbol search and call graph traversal."
        )]
        fn symlens_index_workspace(
            &self,
            Parameters(params): Parameters<IndexWorkspaceParams>,
        ) -> Result<String, McpError> {
            let roots: Vec<RootInfo> = params
                .roots
                .iter()
                .filter_map(|p| PathBuf::from(p).canonicalize().ok())
                .map(RootInfo::new)
                .collect();

            if roots.is_empty() {
                return Err(mcp_error("No valid root paths provided"));
            }

            let _force = params.force.unwrap_or(false);

            match indexer::index_workspace(&roots, 100_000, None, &indexer::WalkOptions::default())
            {
                Ok(result) => match storage::save_workspace(&result.index) {
                    Ok(cache_path) => {
                        invalidate_key(&result.index.workspace_hash);
                        for r in &roots {
                            invalidate_key(&cache_key(&r.path));
                        }
                        Ok(serde_json::to_string_pretty(&json!({
                            "roots": roots.iter().map(|r| r.path.to_string_lossy()).collect::<Vec<_>>(),
                            "files": result.index.file_symbols.len(),
                            "symbols": result.index.symbols.len(),
                            "duration_ms": result.duration_ms,
                            "cache": cache_path.to_string_lossy(),
                        }))
                        .unwrap_or_default())
                    }
                    Err(e) => Err(mcp_error(format!("Save failed: {e}"))),
                },
                Err(e) => Err(mcp_error(format!("Workspace index failed: {e}"))),
            }
        }

        #[tool(description = "BM25 search symbols by name, signature, or docs.")]
        fn symlens_search(
            &self,
            Parameters(params): Parameters<SearchParams>,
        ) -> Result<String, McpError> {
            let limit = params.limit.unwrap_or(10);
            let root = PathBuf::from(&params.path);

            let provider = load_provider(&root)
                .ok_or_else(|| mcp_error("No index found. Run symlens_index first."))?;

            let results = if let Ok(Some(engine)) = provider.open_search() {
                match engine.search(&params.query, limit * 2) {
                    Ok(search_results) => {
                        let mut syms: Vec<_> = search_results
                            .iter()
                            .filter_map(|r| {
                                let id = crate::model::symbol::SymbolId(r.symbol_id.clone());
                                provider.get(&id).map(|s| (s, r.score))
                            })
                            .collect();
                        if let Some(ref kf) = params.kind
                            && let Some(kind) = SymbolKind::from_str(kf)
                        {
                            syms.retain(|(s, _)| s.kind == kind);
                        }
                        syms.truncate(limit);
                        syms
                    }
                    Err(_) => provider
                        .search(&params.query, limit)
                        .into_iter()
                        .map(|s| (s, 0.0f32))
                        .collect(),
                }
            } else {
                provider
                    .search(&params.query, limit)
                    .into_iter()
                    .map(|s| (s, 0.0f32))
                    .collect()
            };

            let items = fmt::format_search_results(&results);

            Ok(
                serde_json::to_string_pretty(&json!({ "results": items, "count": items.len() }))
                    .unwrap_or_default(),
            )
        }

        #[tool(description = "Get detailed info about a symbol by ID.")]
        fn symlens_symbol(
            &self,
            Parameters(params): Parameters<SymbolParams>,
        ) -> Result<String, McpError> {
            let include_source = params.source.unwrap_or(false);
            let root = PathBuf::from(&params.path);

            let provider = load_provider(&root).ok_or_else(|| mcp_error("No index found."))?;

            let id = crate::model::symbol::SymbolId(params.symbol_id.clone());
            let symbol = provider
                .get(&id)
                .ok_or_else(|| mcp_error(format!("Symbol not found: {}", params.symbol_id)))?;

            let source = if include_source {
                // Resolve root_id: SymbolId stores label (e.g. "audio"), but
                // resolve_absolute needs the hash id (e.g. "f270c23c").
                let label = symbol.id.root_id();
                let resolved_root_id = if label.is_empty() {
                    ""
                } else {
                    provider
                        .roots()
                        .iter()
                        .find(|(_, _, _, lbl)| *lbl == label)
                        .map(|(id, _, _, _)| *id)
                        .unwrap_or("")
                };
                let abs = provider.resolve_absolute(resolved_root_id, &symbol.file_path);
                if abs.exists() {
                    std::fs::read_to_string(&abs).ok().map(|content| {
                        let lines: Vec<&str> = content.lines().collect();
                        let start = (symbol.span.start_line as usize).saturating_sub(1);
                        let end = (symbol.span.end_line as usize).min(lines.len());
                        lines[start..end].join("\n")
                    })
                } else {
                    None
                }
            } else {
                None
            };

            let result = fmt::format_symbol_value(symbol, source.as_deref());

            Ok(serde_json::to_string_pretty(&result).unwrap_or_default())
        }

        #[tool(description = "Get file or project outline.")]
        fn symlens_outline(
            &self,
            Parameters(params): Parameters<OutlineParams>,
        ) -> Result<String, McpError> {
            let root = PathBuf::from(&params.path);

            let provider = load_provider(&root).ok_or_else(|| mcp_error("No index found."))?;

            let value = if let Some(file_path) = &params.file {
                let path = PathBuf::from(file_path);
                // Try exact FileKey display match first ("[label]path"), then
                // fall back to relative-path-only match.
                let keys = provider.file_keys();
                let file_keys: Vec<&crate::model::project::FileKey> = keys
                    .iter()
                    .filter(|fk| fk.path == path || fk.display() == *file_path)
                    .collect();

                let mut all_items: Vec<serde_json::Value> = Vec::new();
                for fk in &file_keys {
                    let symbols = provider.symbols_in_file(fk);
                    for s in symbols {
                        all_items.push(json!({
                            "id": s.id.0, "name": s.name, "kind": s.kind.as_str(),
                            "lines": [s.span.start_line, s.span.end_line], "signature": s.signature,
                        }));
                    }
                }
                // If no keys matched, return empty result (not an error)
                json!({ "file": file_path, "symbols": all_items, "count": all_items.len() })
            } else {
                let stats = provider.stats();
                let files: Vec<serde_json::Value> = provider
                    .file_keys()
                    .iter()
                    .map(|fk| {
                        let syms = provider.symbols_in_file(fk);
                        json!({ "file": fk.path.to_string_lossy(), "symbol_count": syms.len() })
                    })
                    .collect();
                json!({
                    "files": files, "total_files": stats.total_files,
                    "total_symbols": stats.total_symbols, "by_language": stats.by_language,
                })
            };

            Ok(serde_json::to_string_pretty(&value).unwrap_or_default())
        }

        #[tool(description = "Find references to a symbol using pre-computed index.")]
        fn symlens_refs(
            &self,
            Parameters(params): Parameters<RefsParams>,
        ) -> Result<String, McpError> {
            let limit = params.limit.unwrap_or(50);
            let root = PathBuf::from(&params.path);

            let provider = load_provider(&root).ok_or_else(|| mcp_error("No index found."))?;

            let target_kind = params
                .kind
                .as_deref()
                .and_then(crate::parser::traits::RefKind::from_filter_str);

            let (refs, files, total) = provider.collect_refs(&params.name, target_kind, limit);
            let ref_items: Vec<serde_json::Value> = refs
                .iter()
                .zip(files.iter())
                .map(|(r, file)| {
                    json!({
                        "file": file.to_string_lossy(),
                        "line": r.line,
                        "context": r.context,
                        "kind": format!("{:?}", r.kind),
                    })
                })
                .collect();

            Ok(serde_json::to_string_pretty(
                &json!({ "name": params.name, "refs": ref_items, "count": total }),
            )
            .unwrap_or_default())
        }

        #[tool(description = "Blast radius analysis: who depends on this symbol?")]
        fn symlens_impact(
            &self,
            Parameters(params): Parameters<ImpactParams>,
        ) -> Result<String, McpError> {
            let depth = params.depth.unwrap_or(3);
            let root = PathBuf::from(&params.path);

            let provider = load_provider(&root).ok_or_else(|| mcp_error("No index found."))?;

            let graph = provider
                .call_graph()
                .ok_or_else(|| mcp_error("No call graph. Re-run symlens_index."))?;

            let result = crate::graph::impact::analyze_impact(graph, &params.name, depth);

            Ok(serde_json::to_string_pretty(&json!({
                "target": result.target,
                "direct_callers": result.direct_callers,
                "direct_callees": result.direct_callees,
                "transitive_callers": result.transitive_callers.iter()
                    .map(|(n, d)| json!({"name": n, "depth": d}))
                    .collect::<Vec<_>>(),
                "transitive_callees": result.transitive_callees.iter()
                    .map(|(n, d)| json!({"name": n, "depth": d}))
                    .collect::<Vec<_>>(),
                "total_dependents": result.direct_callers.len() + result.transitive_callers.len(),
                "affected_modules": result.affected_modules,
                "has_cycle": result.has_cycle,
                "risk_score": format!("{:.2}", result.risk_score),
            }))
            .unwrap_or_default())
        }

        #[tool(description = "Show direct callers of a symbol (who calls this?).")]
        fn symlens_callers(
            &self,
            Parameters(params): Parameters<CallersParams>,
        ) -> Result<String, McpError> {
            let limit = params.limit.unwrap_or(20);
            let root = PathBuf::from(&params.path);

            let provider = load_provider(&root).ok_or_else(|| mcp_error("No index found."))?;

            let graph = provider
                .call_graph()
                .ok_or_else(|| mcp_error("No call graph. Re-run symlens_index."))?;

            let names = graph.callers(&params.name);
            let items = fmt::enrich_callers_json(&names, limit, &provider);

            Ok(serde_json::to_string_pretty(
                &json!({ "symbol": params.name, "callers": items, "count": names.len() }),
            )
            .unwrap_or_default())
        }

        #[tool(description = "Show direct callees of a symbol (what does this call?).")]
        fn symlens_callees(
            &self,
            Parameters(params): Parameters<CalleesParams>,
        ) -> Result<String, McpError> {
            let limit = params.limit.unwrap_or(20);
            let root = PathBuf::from(&params.path);

            let provider = load_provider(&root).ok_or_else(|| mcp_error("No index found."))?;

            let graph = provider
                .call_graph()
                .ok_or_else(|| mcp_error("No call graph. Re-run symlens_index."))?;

            let names = graph.callees(&params.name);
            let items = fmt::enrich_callers_json(&names, limit, &provider);

            Ok(serde_json::to_string_pretty(
                &json!({ "symbol": params.name, "callees": items, "count": names.len() }),
            )
            .unwrap_or_default())
        }

        #[tool(description = "Read specific lines from a file. Returns line-numbered source code.")]
        fn symlens_lines(
            &self,
            Parameters(params): Parameters<LinesParams>,
        ) -> Result<String, McpError> {
            let root = PathBuf::from(&params.path);
            let full_path = root.join(&params.file);

            // Prevent path traversal: canonicalize must stay under root
            let canonical_root = root
                .canonicalize()
                .map_err(|e| mcp_error(format!("Invalid root: {e}")))?;
            let canonical_path = full_path
                .canonicalize()
                .map_err(|_| mcp_error(format!("File not found: {}", params.file)))?;
            if !canonical_path.starts_with(&canonical_root) {
                return Err(mcp_error("File path escapes project root"));
            }

            let content = std::fs::read_to_string(&canonical_path)
                .map_err(|e| mcp_error(format!("Read failed: {e}")))?;
            let lines: Vec<&str> = content.lines().collect();

            let start = (params.start as usize).saturating_sub(1);
            let end = (params.end as usize).min(lines.len());

            if start >= lines.len() {
                return Err(mcp_error(format!(
                    "Start line {} exceeds file length {}",
                    params.start,
                    lines.len()
                )));
            }

            let max_lines = 500;
            let actual_end = end.min(start + max_lines);

            let line_items: Vec<serde_json::Value> = lines[start..actual_end]
                .iter()
                .enumerate()
                .map(|(i, line)| json!({ "line": start + i + 1, "content": line }))
                .collect();

            let mut result = json!({
                "file": params.file,
                "start": params.start,
                "end": params.end,
                "lines": line_items,
                "total_lines": lines.len(),
            });

            if actual_end < end {
                result["truncated"] = serde_json::Value::Bool(true);
                result["truncated_after"] =
                    serde_json::Value::Number(serde_json::Number::from(actual_end));
            }

            Ok(serde_json::to_string_pretty(&result).unwrap_or_default())
        }

        #[tool(
            description = "Show changed symbols between two git refs. Identifies which functions, structs, etc. were added, modified, or deleted."
        )]
        fn symlens_diff(
            &self,
            Parameters(params): Parameters<DiffParams>,
        ) -> Result<String, McpError> {
            let root = PathBuf::from(&params.path);

            let diff_result = crate::commands::diff::collect_changes(
                &root,
                &params.from,
                &params.to,
                params.kind.as_deref(),
            )
            .map_err(|e| mcp_error(format!("Diff failed: {e}")))?;

            let items: Vec<serde_json::Value> = diff_result
                .changes
                .iter()
                .map(|s| {
                    json!({
                        "file": s.file,
                        "name": s.name,
                        "kind": s.kind.as_str(),
                        "change": match s.change_kind {
                            crate::commands::diff::ChangeKind::Added => "added",
                            crate::commands::diff::ChangeKind::Modified => "modified",
                            crate::commands::diff::ChangeKind::Deleted => "deleted",
                        },
                        "lines": [s.span_start, s.span_end],
                        "signature": s.signature,
                    })
                })
                .collect();

            Ok(serde_json::to_string_pretty(&json!({
                "from": params.from,
                "to": params.to,
                "changes": items,
                "total": diff_result.changes.len(),
                "added": diff_result.added_count,
                "modified": diff_result.modified_count,
                "deleted": diff_result.deleted_count,
            }))
            .unwrap_or_default())
        }

        #[tool(
            description = "Get project index statistics: file counts, symbol counts by language and kind."
        )]
        fn symlens_stats(
            &self,
            Parameters(params): Parameters<StatsParams>,
        ) -> Result<String, McpError> {
            let root = PathBuf::from(&params.path);

            let provider = load_provider(&root).ok_or_else(|| mcp_error("No index found."))?;

            let stats = provider.stats();

            Ok(serde_json::to_string_pretty(&json!({
                "version": provider.version(),
                "indexed_at": provider.indexed_at(),
                "is_workspace": provider.is_workspace(),
                "files": stats.total_files,
                "symbols": stats.total_symbols,
                "by_language": stats.by_language,
                "by_kind": stats.by_kind,
            }))
            .unwrap_or_default())
        }
    }

    #[tool_handler(name = "symlens-mcp")]
    impl ServerHandler for SymLensMcp {}

    // ─── Server entry point ──────────────────────────────────────────

    /// Start the MCP server on stdio transport.
    pub async fn run_mcp_server() -> anyhow::Result<()> {
        let service = SymLensMcp.serve(rmcp::transport::stdio()).await?;
        service.waiting().await?;
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::path::PathBuf;

        fn test_index_provider() -> Arc<IndexProvider> {
            let root = PathBuf::from("/tmp/symlens_mcp_test");
            let index = crate::model::project::ProjectIndex::new(root.clone());
            Arc::new(IndexProvider::from_single(root, index))
        }

        #[test]
        fn cache_key_deterministic() {
            let path = PathBuf::from("/tmp/test_project");
            let k1 = cache_key(&path);
            let k2 = cache_key(&path);
            assert_eq!(k1, k2);
            assert_eq!(k1.len(), 16);
        }

        #[test]
        fn cache_key_different_paths() {
            let k1 = cache_key(PathBuf::from("/tmp/a").as_path());
            let k2 = cache_key(PathBuf::from("/tmp/b").as_path());
            assert_ne!(k1, k2);
        }

        #[test]
        fn invalidate_key_removes_specific() {
            INDEX_CACHE
                .write()
                .insert("test_key".to_string(), test_index_provider());
            INDEX_CACHE
                .write()
                .insert("other_key".to_string(), test_index_provider());
            assert!(INDEX_CACHE.read().contains_key("test_key"));
            assert!(INDEX_CACHE.read().contains_key("other_key"));
            invalidate_key("test_key");
            assert!(!INDEX_CACHE.read().contains_key("test_key"));
            assert!(INDEX_CACHE.read().contains_key("other_key"));
            invalidate_key("other_key");
        }

        #[test]
        fn mcp_error_format() {
            let err: McpError = mcp_error("something went wrong");
            assert!(format!("{:?}", err).contains("something went wrong"));
        }
    }
}
