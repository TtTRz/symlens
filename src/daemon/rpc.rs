use crate::commands::IndexProvider;
use crate::daemon::SharedIndex;
use crate::graph::impact;
use crate::model::symbol::SymbolKind;
use crate::output::json as fmt;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct RpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Serialize)]
struct RpcResponse {
    jsonrpc: &'static str,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Serialize)]
struct RpcError {
    code: i32,
    message: String,
}

pub fn handle_request(line: &str, index: &SharedIndex) -> String {
    let req: RpcRequest = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => {
            return serde_json::to_string(&RpcResponse {
                jsonrpc: "2.0",
                id: 0,
                result: None,
                error: Some(RpcError {
                    code: -32700,
                    message: format!("Parse error: {}", e),
                }),
            })
            .unwrap_or_default();
        }
    };

    let provider = index.read();

    let result = match req.method.as_str() {
        "search" => handle_search(&provider, &req.params),
        "refs" => handle_refs(&provider, &req.params),
        "callers" => handle_callers(&provider, &req.params),
        "callees" => handle_callees(&provider, &req.params),
        "outline" => handle_outline(&provider, &req.params),
        "symbol" => handle_symbol(&provider, &req.params),
        "impact" => handle_impact(&provider, &req.params),
        "status" => handle_status(&provider),
        _ => Err(RpcError {
            code: -32601,
            message: format!("Method not found: {}", req.method),
        }),
    };

    let (result_val, error_val) = match result {
        Ok(v) => (Some(v), None),
        Err(e) => (None, Some(e)),
    };

    serde_json::to_string(&RpcResponse {
        jsonrpc: "2.0",
        id: req.id,
        result: result_val,
        error: error_val,
    })
    .unwrap_or_default()
}

fn handle_search(
    provider: &IndexProvider,
    params: &serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let query = params["query"].as_str().unwrap_or("");
    let limit = params["limit"].as_u64().unwrap_or(20) as usize;

    let results = if let Ok(Some(engine)) = provider.open_search() {
        match engine.search(query, limit * 2) {
            Ok(search_results) => {
                let mut syms: Vec<_> = search_results
                    .iter()
                    .filter_map(|r| {
                        let id = crate::model::symbol::SymbolId(r.symbol_id.clone());
                        provider.get(&id).map(|s| (s, r.score))
                    })
                    .collect();
                if let Some(kind) = params["kind"].as_str()
                    && let Some(kind_enum) = SymbolKind::from_str(kind)
                {
                    syms.retain(|(s, _)| s.kind == kind_enum);
                }
                syms.truncate(limit);
                syms
            }
            Err(_) => provider
                .search(query, limit)
                .into_iter()
                .map(|s| (s, 0.0f32))
                .collect(),
        }
    } else {
        provider
            .search(query, limit)
            .into_iter()
            .map(|s| (s, 0.0f32))
            .collect()
    };

    let items = fmt::format_search_results(&results);
    Ok(serde_json::json!({
        "query": query,
        "results": items,
        "count": items.len(),
    }))
}

fn handle_refs(
    provider: &IndexProvider,
    params: &serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let name = params["name"].as_str().ok_or_else(|| RpcError {
        code: -32602,
        message: "Missing 'name' parameter".into(),
    })?;
    let limit = params["limit"].as_u64().unwrap_or(50) as usize;
    let kind_filter = params["kind"]
        .as_str()
        .and_then(crate::parser::traits::RefKind::from_filter_str);

    let (refs, files, total) = provider.collect_refs(name, kind_filter, limit);
    let ref_items: Vec<serde_json::Value> = refs
        .iter()
        .zip(files.iter())
        .map(|(r, file)| {
            serde_json::json!({
                "file": file.to_string_lossy(),
                "line": r.line,
                "context": r.context,
                "kind": format!("{:?}", r.kind),
            })
        })
        .collect();

    Ok(serde_json::json!({
        "name": name,
        "refs": ref_items,
        "count": total,
    }))
}

fn handle_callers(
    provider: &IndexProvider,
    params: &serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let name = params["name"].as_str().ok_or_else(|| RpcError {
        code: -32602,
        message: "Missing 'name' parameter".into(),
    })?;
    let limit = params["limit"].as_u64().unwrap_or(20) as usize;

    let graph = provider.call_graph().ok_or_else(|| RpcError {
        code: -32603,
        message: "No call graph in index".into(),
    })?;

    let names = graph.callers(name);
    let items = fmt::enrich_callers_json(&names, limit, provider);

    Ok(serde_json::json!({
        "symbol": name,
        "callers": items,
        "count": names.len(),
    }))
}

fn handle_callees(
    provider: &IndexProvider,
    params: &serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let name = params["name"].as_str().ok_or_else(|| RpcError {
        code: -32602,
        message: "Missing 'name' parameter".into(),
    })?;
    let limit = params["limit"].as_u64().unwrap_or(20) as usize;

    let graph = provider.call_graph().ok_or_else(|| RpcError {
        code: -32603,
        message: "No call graph in index".into(),
    })?;

    let names = graph.callees(name);
    let items = fmt::enrich_callers_json(&names, limit, provider);

    Ok(serde_json::json!({
        "symbol": name,
        "callees": items,
        "count": names.len(),
    }))
}

fn handle_outline(
    provider: &IndexProvider,
    params: &serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    if let Some(file) = params["file"].as_str() {
        let path = std::path::PathBuf::from(file);
        let keys = provider.file_keys();
        let file_keys: Vec<&crate::model::project::FileKey> = keys
            .iter()
            .filter(|fk| fk.path == path || fk.display() == file)
            .collect();

        let mut all_symbols: Vec<serde_json::Value> = Vec::new();
        for fk in &file_keys {
            for s in provider.symbols_in_file(fk) {
                all_symbols.push(serde_json::json!({
                    "id": s.id.0,
                    "name": s.name,
                    "kind": s.kind.as_str(),
                    "lines": [s.span.start_line, s.span.end_line],
                    "signature": s.signature,
                }));
            }
        }

        Ok(serde_json::json!({
            "file": file,
            "symbols": all_symbols,
            "count": all_symbols.len(),
        }))
    } else {
        let stats = provider.stats();
        let files: Vec<serde_json::Value> = provider
            .file_keys()
            .iter()
            .map(|fk| {
                let syms = provider.symbols_in_file(fk);
                serde_json::json!({
                    "file": fk.path.to_string_lossy(),
                    "symbol_count": syms.len(),
                })
            })
            .collect();

        Ok(serde_json::json!({
            "files": files,
            "total_files": stats.total_files,
            "total_symbols": stats.total_symbols,
            "by_language": stats.by_language,
        }))
    }
}

fn handle_symbol(
    provider: &IndexProvider,
    params: &serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let symbol_id = params["symbol_id"].as_str().ok_or_else(|| RpcError {
        code: -32602,
        message: "Missing 'symbol_id' parameter".into(),
    })?;

    let sym = provider.find_symbol(symbol_id).ok_or_else(|| RpcError {
        code: -32602,
        message: format!("Symbol not found: {}", symbol_id),
    })?;

    let source = params["source"].as_bool().unwrap_or(false);
    let source_val = if source {
        let label = sym.id.root_id();
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
        let abs = provider.resolve_absolute(resolved_root_id, &sym.file_path);
        if abs.exists() {
            std::fs::read_to_string(&abs).ok().map(|content| {
                let lines: Vec<&str> = content.lines().collect();
                let start = (sym.span.start_line as usize).saturating_sub(1);
                let end = (sym.span.end_line as usize).min(lines.len());
                lines[start..end].join("\n")
            })
        } else {
            None
        }
    } else {
        None
    };

    Ok(fmt::format_symbol_value(sym, source_val.as_deref()))
}

fn handle_impact(
    provider: &IndexProvider,
    params: &serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let name = params["name"].as_str().ok_or_else(|| RpcError {
        code: -32602,
        message: "Missing 'name' parameter".into(),
    })?;
    let depth = params["depth"].as_u64().unwrap_or(3) as usize;

    let graph = provider.call_graph().ok_or_else(|| RpcError {
        code: -32603,
        message: "No call graph in index".into(),
    })?;

    let result = impact::analyze_impact(graph, name, depth);

    Ok(serde_json::json!({
        "target": result.target,
        "direct_callers": result.direct_callers,
        "direct_callees": result.direct_callees,
        "transitive_callers": result.transitive_callers.iter().map(|(n, d)| serde_json::json!({"name": n, "depth": d})).collect::<Vec<_>>(),
        "transitive_callees": result.transitive_callees.iter().map(|(n, d)| serde_json::json!({"name": n, "depth": d})).collect::<Vec<_>>(),
        "affected_modules": result.affected_modules,
        "has_cycle": result.has_cycle,
        "risk_score": result.risk_score,
    }))
}

fn handle_status(provider: &IndexProvider) -> Result<serde_json::Value, RpcError> {
    let stats = provider.stats();
    Ok(serde_json::json!({
        "version": provider.version(),
        "indexed_at": provider.indexed_at(),
        "is_workspace": provider.is_workspace(),
        "total_files": stats.total_files,
        "total_symbols": stats.total_symbols,
        "by_language": stats.by_language,
        "by_kind": stats.by_kind,
        "pid": std::process::id(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::RwLock;
    use std::sync::Arc;

    #[test]
    fn test_parse_rpc_request() {
        let line = r#"{"jsonrpc":"2.0","id":1,"method":"status","params":{}}"#;
        let req: RpcRequest = serde_json::from_str(line).unwrap();
        assert_eq!(req.id, 1);
        assert_eq!(req.method, "status");
    }

    #[test]
    fn test_unknown_method_returns_error() {
        let index = Arc::new(RwLock::new(crate::commands::IndexProvider::from_single(
            std::path::PathBuf::from("/tmp/test"),
            crate::model::project::ProjectIndex::new(std::path::PathBuf::from("/tmp/test")),
        )));
        let line = r#"{"jsonrpc":"2.0","id":2,"method":"nonexistent","params":{}}"#;
        let resp = handle_request(line, &index);
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert!(parsed["error"].is_object());
        assert_eq!(parsed["error"]["code"], -32601);
    }

    #[test]
    fn test_status_handler() {
        let index = Arc::new(RwLock::new(crate::commands::IndexProvider::from_single(
            std::path::PathBuf::from("/tmp/test"),
            crate::model::project::ProjectIndex::new(std::path::PathBuf::from("/tmp/test")),
        )));
        let line = r#"{"jsonrpc":"2.0","id":1,"method":"status","params":{}}"#;
        let resp = handle_request(line, &index);
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert!(parsed["result"].is_object());
        assert_eq!(parsed["result"]["is_workspace"], false);
    }

    #[test]
    fn test_search_handler_empty() {
        let index = Arc::new(RwLock::new(crate::commands::IndexProvider::from_single(
            std::path::PathBuf::from("/tmp/test"),
            crate::model::project::ProjectIndex::new(std::path::PathBuf::from("/tmp/test")),
        )));
        let line = r#"{"jsonrpc":"2.0","id":3,"method":"search","params":{"query":"foo"}}"#;
        let resp = handle_request(line, &index);
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert!(parsed["result"].is_object());
        assert_eq!(parsed["result"]["count"], 0);
    }

    #[test]
    fn test_refs_missing_name_returns_error() {
        let index = Arc::new(RwLock::new(crate::commands::IndexProvider::from_single(
            std::path::PathBuf::from("/tmp/test"),
            crate::model::project::ProjectIndex::new(std::path::PathBuf::from("/tmp/test")),
        )));
        let line = r#"{"jsonrpc":"2.0","id":4,"method":"refs","params":{}}"#;
        let resp = handle_request(line, &index);
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(parsed["error"]["code"], -32602);
        assert!(
            parsed["error"]["message"]
                .as_str()
                .unwrap()
                .contains("name")
        );
    }

    #[test]
    fn test_callers_no_call_graph_returns_error() {
        let index = Arc::new(RwLock::new(crate::commands::IndexProvider::from_single(
            std::path::PathBuf::from("/tmp/test"),
            crate::model::project::ProjectIndex::new(std::path::PathBuf::from("/tmp/test")),
        )));
        let line = r#"{"jsonrpc":"2.0","id":5,"method":"callers","params":{"name":"foo"}}"#;
        let resp = handle_request(line, &index);
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(parsed["error"]["code"], -32603);
    }

    #[test]
    fn test_outline_empty_project() {
        let index = Arc::new(RwLock::new(crate::commands::IndexProvider::from_single(
            std::path::PathBuf::from("/tmp/test"),
            crate::model::project::ProjectIndex::new(std::path::PathBuf::from("/tmp/test")),
        )));
        let line = r#"{"jsonrpc":"2.0","id":6,"method":"outline","params":{}}"#;
        let resp = handle_request(line, &index);
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(parsed["result"]["total_files"], 0);
        assert_eq!(parsed["result"]["total_symbols"], 0);
    }

    #[test]
    fn test_refs_with_kind_filter_no_match() {
        let index = Arc::new(RwLock::new(crate::commands::IndexProvider::from_single(
            std::path::PathBuf::from("/tmp/test"),
            crate::model::project::ProjectIndex::new(std::path::PathBuf::from("/tmp/test")),
        )));
        // kind="unknown_kind" → unrecognized, so target_kind is None → no filtering applied
        let line = r#"{"jsonrpc":"2.0","id":7,"method":"refs","params":{"name":"foo","kind":"unknown_kind"}}"#;
        let resp = handle_request(line, &index);
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        // No refs found (empty index), but no error either
        assert_eq!(parsed["result"]["count"], 0);
    }
}
