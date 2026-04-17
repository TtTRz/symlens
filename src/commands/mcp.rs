#[cfg(feature = "mcp")]
pub mod server {
    use crate::index::{indexer, storage};
    use crate::model::project::RootInfo;
    use crate::model::symbol::SymbolKind;
    use crate::output::json as fmt;
    use rayon::prelude::*;
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
    use std::sync::{Arc, LazyLock, RwLock};

    use crate::model::project::ProjectIndex;
    use crate::model::workspace::WorkspaceIndex;

    // ─── Static index cache ──────────────────────────────────────────

    /// Per-root cache: key is root_hash (blake3[..16]).
    static SINGLE_CACHE: LazyLock<RwLock<HashMap<String, Arc<ProjectIndex>>>> =
        LazyLock::new(|| RwLock::new(HashMap::new()));

    /// Workspace cache: key is workspace_hash (blake3[..16] with "ws_" prefix).
    static WORKSPACE_CACHE: LazyLock<RwLock<HashMap<String, Arc<WorkspaceIndex>>>> =
        LazyLock::new(|| RwLock::new(HashMap::new()));

    fn load_or_cache(root: &std::path::Path) -> Option<Arc<ProjectIndex>> {
        let key = blake3::hash(root.to_string_lossy().as_bytes()).to_hex()[..16].to_string();
        if let Some(idx) = SINGLE_CACHE.read().expect("cache lock poisoned").get(&key) {
            return Some(Arc::clone(idx));
        }
        if let Ok(Some(idx)) = storage::load(root) {
            let arc = Arc::new(idx);
            SINGLE_CACHE.write().unwrap().insert(key, Arc::clone(&arc));
            Some(arc)
        } else {
            None
        }
    }

    fn load_or_cache_workspace(roots: &[RootInfo]) -> Option<Arc<WorkspaceIndex>> {
        let key = storage::compute_workspace_hash(roots);
        if let Some(idx) = WORKSPACE_CACHE
            .read()
            .expect("cache lock poisoned")
            .get(&key)
        {
            return Some(Arc::clone(idx));
        }
        if let Ok(Some(idx)) = storage::load_workspace(roots) {
            let arc = Arc::new(idx);
            WORKSPACE_CACHE
                .write()
                .unwrap()
                .insert(key, Arc::clone(&arc));
            Some(arc)
        } else {
            None
        }
    }

    fn invalidate_single(key: &str) {
        SINGLE_CACHE
            .write()
            .expect("cache lock poisoned")
            .remove(key);
    }

    fn invalidate_workspace(key: &str) {
        WORKSPACE_CACHE
            .write()
            .expect("cache lock poisoned")
            .remove(key);
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

            match indexer::index_project_incremental(&root, 100_000, prev_index.as_ref()) {
                Ok(result) => match storage::save(&result.index) {
                    Ok(cache_path) => {
                        let key = blake3::hash(root.to_string_lossy().as_bytes()).to_hex()[..16]
                            .to_string();
                        invalidate_single(&key);
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

            let force = params.force.unwrap_or(false);

            let prev_workspace = if force {
                None
            } else {
                load_or_cache_workspace(&roots)
            };

            match indexer::index_workspace(&roots, 100_000, prev_workspace.as_deref()) {
                Ok(result) => match storage::save_workspace(&result.index) {
                    Ok(cache_path) => {
                        let key = storage::compute_workspace_hash(&roots);
                        invalidate_workspace(&key);
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

            let index = load_or_cache(&root)
                .ok_or_else(|| mcp_error("No index found. Run symlens_index first."))?;

            let results = if let Ok(Some(engine)) = storage::open_search(&root) {
                match engine.search(&params.query, limit * 2) {
                    Ok(search_results) => {
                        let mut syms: Vec<_> = search_results
                            .iter()
                            .filter_map(|r| {
                                let id = crate::model::symbol::SymbolId(r.symbol_id.clone());
                                index.get(&id).map(|s| (s, r.score))
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
                    Err(_) => index
                        .search(&params.query, limit)
                        .into_iter()
                        .map(|s| (s, 0.0f32))
                        .collect(),
                }
            } else {
                index
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

            let index = load_or_cache(&root).ok_or_else(|| mcp_error("No index found."))?;

            let id = crate::model::symbol::SymbolId(params.symbol_id.clone());
            let symbol = index
                .get(&id)
                .ok_or_else(|| mcp_error(format!("Symbol not found: {}", params.symbol_id)))?;

            let source = if include_source {
                let source_file = root.join(&symbol.file_path);
                std::fs::read_to_string(&source_file).ok().map(|content| {
                    let lines: Vec<&str> = content.lines().collect();
                    let start = (symbol.span.start_line as usize).saturating_sub(1);
                    let end = (symbol.span.end_line as usize).min(lines.len());
                    lines[start..end].join("\n")
                })
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

            let index = load_or_cache(&root).ok_or_else(|| mcp_error("No index found."))?;

            let value = if let Some(file_path) = &params.file {
                let fp = PathBuf::from(file_path);
                let symbols = index.symbols_in_file(&fp);
                let items: Vec<serde_json::Value> = symbols
                    .iter()
                    .map(|s| {
                        json!({
                            "id": s.id.0, "name": s.name, "kind": s.kind.as_str(),
                            "lines": [s.span.start_line, s.span.end_line], "signature": s.signature,
                        })
                    })
                    .collect();
                json!({ "file": file_path, "symbols": items, "count": items.len() })
            } else {
                let stats = index.stats();
                let files: Vec<serde_json::Value> = index
                    .file_symbols
                    .iter()
                    .map(|(file, ids)| json!({ "file": file.to_string_lossy(), "symbols": ids.len() }))
                    .collect();
                json!({
                    "files": files, "total_files": stats.total_files,
                    "total_symbols": stats.total_symbols, "by_language": stats.by_language,
                })
            };

            Ok(serde_json::to_string_pretty(&value).unwrap_or_default())
        }

        #[tool(description = "Find references to a symbol.")]
        fn symlens_refs(
            &self,
            Parameters(params): Parameters<RefsParams>,
        ) -> Result<String, McpError> {
            let root = PathBuf::from(&params.path);

            let index = load_or_cache(&root).ok_or_else(|| mcp_error("No index found."))?;

            let candidate_files: Vec<PathBuf> =
                if let Some(importing_files) = index.import_names.get(&params.name) {
                    let mut files: std::collections::HashSet<PathBuf> =
                        importing_files.iter().cloned().collect();
                    for sym in index.symbols.values() {
                        if sym.name == params.name {
                            files.insert(sym.file_path.clone());
                        }
                    }
                    files.into_iter().collect()
                } else {
                    index.file_symbols.keys().cloned().collect()
                };

            let name_owned = params.name.clone();
            let all_refs: Vec<serde_json::Value> = candidate_files
                .par_iter()
                .flat_map_iter(|file_path| {
                    let full_path = root.join(file_path);
                    let mut results = Vec::new();
                    if !full_path.exists() {
                        return results;
                    }
                    let registry = crate::parser::registry::LanguageRegistry::new();
                    if let Some(parser) = registry.parser_for(&full_path)
                        && let Ok(source) = std::fs::read(&full_path)
                        && let Ok(refs) = parser.find_identifiers(&source, &name_owned)
                    {
                        for r in refs {
                            if r.kind != crate::parser::traits::RefKind::Definition {
                                results.push(json!({
                                    "file": file_path.to_string_lossy(), "line": r.line,
                                    "context": r.context, "kind": format!("{:?}", r.kind),
                                }));
                            }
                        }
                    }
                    results
                })
                .collect();

            Ok(serde_json::to_string_pretty(
                &json!({ "name": params.name, "refs": all_refs, "count": all_refs.len() }),
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

            let index = load_or_cache(&root).ok_or_else(|| mcp_error("No index found."))?;

            let graph = index
                .call_graph
                .as_ref()
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

            let index = load_or_cache(&root).ok_or_else(|| mcp_error("No index found."))?;

            let graph = index
                .call_graph
                .as_ref()
                .ok_or_else(|| mcp_error("No call graph. Re-run symlens_index."))?;

            let callers: Vec<&str> = graph
                .callers(&params.name)
                .into_iter()
                .take(limit)
                .collect();
            Ok(serde_json::to_string_pretty(
                &json!({ "name": params.name, "callers": callers, "count": callers.len() }),
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

            let index = load_or_cache(&root).ok_or_else(|| mcp_error("No index found."))?;

            let graph = index
                .call_graph
                .as_ref()
                .ok_or_else(|| mcp_error("No call graph. Re-run symlens_index."))?;

            let callees: Vec<&str> = graph
                .callees(&params.name)
                .into_iter()
                .take(limit)
                .collect();
            Ok(serde_json::to_string_pretty(
                &json!({ "name": params.name, "callees": callees, "count": callees.len() }),
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

            if !full_path.exists() {
                return Err(mcp_error(format!("File not found: {}", params.file)));
            }

            let content = std::fs::read_to_string(&full_path)
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

            let index = load_or_cache(&root).ok_or_else(|| mcp_error("No index found."))?;

            let stats = index.stats();

            Ok(serde_json::to_string_pretty(&json!({
                "root": index.root.to_string_lossy(),
                "version": index.version,
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
}
