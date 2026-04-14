#[cfg(feature = "mcp")]
pub mod server {
    use crate::index::{indexer, storage};
    use crate::model::symbol::SymbolKind;
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

    // ─── Static index cache ──────────────────────────────────────────

    static INDEX_CACHE: LazyLock<RwLock<HashMap<PathBuf, Arc<ProjectIndex>>>> =
        LazyLock::new(|| RwLock::new(HashMap::new()));

    fn load_or_cache(root: &std::path::Path) -> Option<Arc<ProjectIndex>> {
        if let Some(idx) = INDEX_CACHE.read().expect("cache lock poisoned").get(root) {
            return Some(Arc::clone(idx));
        }
        if let Ok(Some(idx)) = storage::load(root) {
            let arc = Arc::new(idx);
            INDEX_CACHE
                .write()
                .unwrap()
                .insert(root.to_path_buf(), Arc::clone(&arc));
            Some(arc)
        } else {
            None
        }
    }

    fn invalidate_cache(root: &std::path::Path) {
        INDEX_CACHE
            .write()
            .expect("cache lock poisoned")
            .remove(root);
    }

    fn mcp_error(msg: impl std::fmt::Display) -> McpError {
        McpError::internal_error(msg.to_string(), None)
    }

    // ─── Parameter structs ───────────────────────────────────────────

    #[derive(Debug, Deserialize, JsonSchema)]
    pub struct IndexParams {
        /// Project root path
        pub path: String,
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

            match indexer::index_project(&root, 100_000) {
                Ok(result) => match storage::save(&result.index) {
                    Ok(cache_path) => {
                        invalidate_cache(&root);
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

            let items: Vec<serde_json::Value> = results
                .iter()
                .map(|(sym, score)| {
                    json!({
                        "id": sym.id.0, "name": sym.name, "kind": sym.kind.as_str(),
                        "file": sym.file_path.to_string_lossy(),
                        "lines": [sym.span.start_line, sym.span.end_line],
                        "signature": sym.signature, "doc": sym.doc_comment, "score": score,
                    })
                })
                .collect();

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

            let mut result = json!({
                "id": symbol.id.0, "name": symbol.name, "qualified_name": symbol.qualified_name,
                "kind": symbol.kind.as_str(), "file": symbol.file_path.to_string_lossy(),
                "lines": [symbol.span.start_line, symbol.span.end_line],
                "signature": symbol.signature, "doc": symbol.doc_comment,
                "visibility": format!("{:?}", symbol.visibility),
            });

            if include_source {
                let source_file = root.join(&symbol.file_path);
                if let Ok(content) = std::fs::read_to_string(&source_file) {
                    let lines: Vec<&str> = content.lines().collect();
                    let start = (symbol.span.start_line as usize).saturating_sub(1);
                    let end = (symbol.span.end_line as usize).min(lines.len());
                    result["source"] = serde_json::Value::String(lines[start..end].join("\n"));
                }
            }

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
