#[cfg(feature = "mcp")]
pub mod server {
    use crate::index::{indexer, storage};
    use crate::model::symbol::SymbolKind;
    use serde_json::{Value, json};
    use std::path::PathBuf;
    use tower_lsp::jsonrpc::{self, Result};
    use tower_lsp::lsp_types::*;
    use tower_lsp::{Client, LanguageServer, LspService, Server};

    pub struct CodeLensMcp {
        client: Client,
    }

    impl CodeLensMcp {
        pub fn new(client: Client) -> Self {
            Self { client }
        }
    }

    #[tower_lsp::async_trait]
    impl LanguageServer for CodeLensMcp {
        async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
            Ok(InitializeResult {
                capabilities: ServerCapabilities::default(),
                server_info: Some(ServerInfo {
                    name: "codelens-mcp".into(),
                    version: Some(env!("CARGO_PKG_VERSION").into()),
                }),
            })
        }

        async fn initialized(&self, _: InitializedParams) {
            self.client
                .log_message(MessageType::INFO, "CodeLens MCP server initialized")
                .await;
        }

        async fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    // ─── MCP tool definitions ───────────────────────────────────────

    fn tool_definitions() -> Vec<Value> {
        vec![
            json!({
                "name": "codelens_index",
                "description": "Index a project directory with tree-sitter. Returns symbol count and timing.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Project root path" }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "codelens_search",
                "description": "BM25 search symbols by name, signature, or docs.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Project root path" },
                        "query": { "type": "string", "description": "Search query" },
                        "limit": { "type": "integer", "description": "Max results (default 10)" },
                        "kind": { "type": "string", "description": "Filter by kind" }
                    },
                    "required": ["path", "query"]
                }
            }),
            json!({
                "name": "codelens_symbol",
                "description": "Get detailed info about a symbol by ID.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Project root path" },
                        "symbol_id": { "type": "string", "description": "Symbol ID" },
                        "source": { "type": "boolean", "description": "Include source code" }
                    },
                    "required": ["path", "symbol_id"]
                }
            }),
            json!({
                "name": "codelens_outline",
                "description": "Get file or project outline.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Project root path" },
                        "file": { "type": "string", "description": "File path (omit for project)" }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "codelens_refs",
                "description": "Find references to a symbol.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Project root path" },
                        "name": { "type": "string", "description": "Symbol name" },
                        "kind": { "type": "string", "description": "Filter by ref kind" }
                    },
                    "required": ["path", "name"]
                }
            }),
            json!({
                "name": "codelens_impact",
                "description": "Blast radius analysis: who depends on this symbol?",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Project root path" },
                        "name": { "type": "string", "description": "Symbol name" },
                        "depth": { "type": "integer", "description": "Max depth (default 3)" }
                    },
                    "required": ["path", "name"]
                }
            }),
        ]
    }

    fn execute_tool(name: &str, args: &Value) -> Value {
        match name {
            "codelens_index" => tool_index(args),
            "codelens_search" => tool_search(args),
            "codelens_symbol" => tool_symbol(args),
            "codelens_outline" => tool_outline(args),
            "codelens_refs" => tool_refs(args),
            "codelens_impact" => tool_impact(args),
            _ => json!({ "error": format!("Unknown tool: {}", name) }),
        }
    }

    // ─── Custom JSON-RPC method handlers (MCP protocol) ─────────────

    impl CodeLensMcp {
        async fn handle_tools_list(&self, _params: Value) -> jsonrpc::Result<Value> {
            Ok(json!({ "tools": tool_definitions() }))
        }

        async fn handle_tools_call(&self, params: Value) -> jsonrpc::Result<Value> {
            let name = params["name"].as_str().unwrap_or("");
            let args = &params["arguments"];
            let result = execute_tool(name, args);

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&result).unwrap_or_default()
                }]
            }))
        }
    }

    // ─── Tool implementations ───────────────────────────────────────

    fn tool_index(args: &Value) -> Value {
        let path = args["path"].as_str().unwrap_or(".");
        let root = PathBuf::from(path);

        match indexer::index_project(&root, 100_000) {
            Ok(result) => match storage::save(&result.index) {
                Ok(cache_path) => json!({
                    "files": result.index.file_symbols.len(),
                    "symbols": result.index.symbols.len(),
                    "duration_ms": result.duration_ms,
                    "cache": cache_path.to_string_lossy(),
                }),
                Err(e) => json!({ "error": format!("Save failed: {}", e) }),
            },
            Err(e) => json!({ "error": format!("Index failed: {}", e) }),
        }
    }

    fn tool_search(args: &Value) -> Value {
        let path = args["path"].as_str().unwrap_or(".");
        let query = args["query"].as_str().unwrap_or("");
        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
        let kind_filter = args["kind"].as_str();
        let root = PathBuf::from(path);

        let index = match storage::load(&root) {
            Ok(Some(idx)) => idx,
            _ => return json!({ "error": "No index found. Run codelens_index first." }),
        };

        let results = if let Ok(Some(engine)) = storage::open_search(&root) {
            match engine.search(query, limit * 2) {
                Ok(search_results) => {
                    let mut syms: Vec<_> = search_results
                        .iter()
                        .filter_map(|r| {
                            let id = crate::model::symbol::SymbolId(r.symbol_id.clone());
                            index.get(&id).map(|s| (s, r.score))
                        })
                        .collect();
                    if let Some(kf) = kind_filter {
                        if let Some(kind) = SymbolKind::from_str(kf) {
                            syms.retain(|(s, _)| s.kind == kind);
                        }
                    }
                    syms.truncate(limit);
                    syms
                }
                Err(_) => index
                    .search(query, limit)
                    .into_iter()
                    .map(|s| (s, 0.0f32))
                    .collect(),
            }
        } else {
            index
                .search(query, limit)
                .into_iter()
                .map(|s| (s, 0.0f32))
                .collect()
        };

        let items: Vec<Value> = results
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

        json!({ "results": items, "count": items.len() })
    }

    fn tool_symbol(args: &Value) -> Value {
        let path = args["path"].as_str().unwrap_or(".");
        let symbol_id = args["symbol_id"].as_str().unwrap_or("");
        let include_source = args["source"].as_bool().unwrap_or(false);
        let root = PathBuf::from(path);

        let index = match storage::load(&root) {
            Ok(Some(idx)) => idx,
            _ => return json!({ "error": "No index found." }),
        };

        let id = crate::model::symbol::SymbolId(symbol_id.to_string());
        let symbol = match index.get(&id) {
            Some(s) => s,
            None => return json!({ "error": format!("Symbol not found: {}", symbol_id) }),
        };

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
                result["source"] = Value::String(lines[start..end].join("\n"));
            }
        }

        result
    }

    fn tool_outline(args: &Value) -> Value {
        let path = args["path"].as_str().unwrap_or(".");
        let file = args["file"].as_str();
        let root = PathBuf::from(path);

        let index = match storage::load(&root) {
            Ok(Some(idx)) => idx,
            _ => return json!({ "error": "No index found." }),
        };

        if let Some(file_path) = file {
            let fp = PathBuf::from(file_path);
            let symbols = index.symbols_in_file(&fp);
            let items: Vec<Value> = symbols
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
            let files: Vec<Value> = index
                .file_symbols
                .iter()
                .map(|(file, ids)| json!({ "file": file.to_string_lossy(), "symbols": ids.len() }))
                .collect();
            json!({
                "files": files, "total_files": stats.total_files,
                "total_symbols": stats.total_symbols, "by_language": stats.by_language,
            })
        }
    }

    fn tool_refs(args: &Value) -> Value {
        let path = args["path"].as_str().unwrap_or(".");
        let name = args["name"].as_str().unwrap_or("");
        let root = PathBuf::from(path);

        let index = match storage::load(&root) {
            Ok(Some(idx)) => idx,
            _ => return json!({ "error": "No index found." }),
        };

        let registry = crate::parser::registry::LanguageRegistry::new();
        let candidate_files: Vec<PathBuf> =
            if let Some(importing_files) = index.import_names.get(name) {
                let mut files: std::collections::HashSet<PathBuf> =
                    importing_files.iter().cloned().collect();
                for sym in index.symbols.values() {
                    if sym.name == name {
                        files.insert(sym.file_path.clone());
                    }
                }
                files.into_iter().collect()
            } else {
                index.file_symbols.keys().cloned().collect()
            };

        let mut all_refs = Vec::new();
        for file_path in &candidate_files {
            let full_path = root.join(file_path);
            if !full_path.exists() {
                continue;
            }
            if let Some(parser) = registry.parser_for(&full_path) {
                if let Ok(source) = std::fs::read(&full_path) {
                    if let Ok(refs) = parser.find_identifiers(&source, name) {
                        for r in refs {
                            if r.kind != crate::parser::traits::RefKind::Definition {
                                all_refs.push(json!({
                                    "file": file_path.to_string_lossy(), "line": r.line,
                                    "context": r.context, "kind": format!("{:?}", r.kind),
                                }));
                            }
                        }
                    }
                }
            }
        }

        json!({ "name": name, "refs": all_refs, "count": all_refs.len() })
    }

    fn tool_impact(args: &Value) -> Value {
        let path = args["path"].as_str().unwrap_or(".");
        let name = args["name"].as_str().unwrap_or("");
        let depth = args["depth"].as_u64().unwrap_or(3) as usize;
        let root = PathBuf::from(path);

        let index = match storage::load(&root) {
            Ok(Some(idx)) => idx,
            _ => return json!({ "error": "No index found." }),
        };

        let graph = match index.call_graph.as_ref() {
            Some(g) => g,
            None => return json!({ "error": "No call graph. Re-run codelens_index." }),
        };

        let result = crate::graph::impact::analyze_impact(graph, name, depth);

        json!({
            "target": result.target,
            "direct_callers": result.direct_callers,
            "direct_callees": result.direct_callees,
            "transitive_callers": result.transitive_callers.iter()
                .map(|(n, d)| json!({"name": n, "depth": d}))
                .collect::<Vec<_>>(),
            "total_dependents": result.direct_callers.len() + result.transitive_callers.len(),
        })
    }

    // ─── Server startup ─────────────────────────────────────────────

    /// Start the MCP server on stdio with custom method routing.
    pub async fn run_mcp_server() -> anyhow::Result<()> {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::build(|client| CodeLensMcp::new(client))
            .custom_method("tools/list", CodeLensMcp::handle_tools_list)
            .custom_method("tools/call", CodeLensMcp::handle_tools_call)
            .finish();

        Server::new(stdin, stdout, socket).serve(service).await;
        Ok(())
    }
}
