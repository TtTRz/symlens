use crate::commands::IndexProvider;
use crate::daemon::SharedIndex;
use crate::graph::impact;
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

    let provider = match index.read() {
        Ok(guard) => guard,
        Err(e) => {
            return serde_json::to_string(&RpcResponse {
                jsonrpc: "2.0",
                id: req.id,
                result: None,
                error: Some(RpcError {
                    code: -32603,
                    message: format!("Index lock poisoned: {}", e),
                }),
            })
            .unwrap_or_default();
        }
    };

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
    let results = provider.search(query, limit);

    let kind_filter = params["kind"].as_str();
    let results: Vec<_> = if kind_filter.is_some() {
        results
            .into_iter()
            .filter(|s| kind_filter.is_some_and(|k| s.kind.as_str() == k))
            .map(|s| (s, 0.0f32))
            .collect()
    } else {
        results.into_iter().map(|s| (s, 0.0f32)).collect()
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
    let kind_filter = params["kind"].as_str();

    let target_kind = kind_filter.and_then(|k| match k.to_lowercase().as_str() {
        "call" => Some(crate::parser::traits::RefKind::Call),
        "type" => Some(crate::parser::traits::RefKind::TypeRef),
        "import" | "use" => Some(crate::parser::traits::RefKind::Import),
        "field" => Some(crate::parser::traits::RefKind::FieldAccess),
        "constructor" | "ctor" => Some(crate::parser::traits::RefKind::Constructor),
        _ => None,
    });

    let candidate_keys = provider.identifier_files_for(name);
    let mut refs = Vec::new();
    for file_key in &candidate_keys {
        let idents = provider.identifiers_in_file(file_key);
        for r in idents {
            if r.name == name
                && r.kind != crate::parser::traits::RefKind::Definition
                && target_kind.is_none_or(|tk| r.kind == tk)
            {
                refs.push(serde_json::json!({
                    "file": file_key.path.to_string_lossy(),
                    "line": r.line,
                    "context": r.context,
                    "kind": format!("{:?}", r.kind),
                }));
            }
        }
    }

    let total = refs.len();
    refs.truncate(limit);

    Ok(serde_json::json!({
        "name": name,
        "refs": refs,
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
    let items: Vec<serde_json::Value> = names
        .iter()
        .take(limit)
        .map(|n| {
            if let Some(sym) = provider.find_symbol(n) {
                serde_json::json!({
                    "name": n,
                    "file": sym.file_path.to_string_lossy(),
                    "line": sym.span.start_line,
                    "kind": sym.kind.as_str(),
                    "signature": sym.signature,
                })
            } else {
                serde_json::json!({ "name": n })
            }
        })
        .collect();

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
    let items: Vec<serde_json::Value> = names
        .iter()
        .take(limit)
        .map(|n| {
            if let Some(sym) = provider.find_symbol(n) {
                serde_json::json!({
                    "name": n,
                    "file": sym.file_path.to_string_lossy(),
                    "line": sym.span.start_line,
                    "kind": sym.kind.as_str(),
                    "signature": sym.signature,
                })
            } else {
                serde_json::json!({ "name": n })
            }
        })
        .collect();

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
        // Try all file keys to find one matching the path (works for both single and workspace)
        let file_key = provider
            .file_keys()
            .into_iter()
            .find(|fk| fk.path == path)
            .unwrap_or_else(|| crate::model::project::FileKey::new("", path));

        let symbols: Vec<serde_json::Value> = provider
            .symbols_in_file(&file_key)
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id.0,
                    "name": s.name,
                    "kind": s.kind.as_str(),
                    "lines": [s.span.start_line, s.span.end_line],
                    "signature": s.signature,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "file": file,
            "symbols": symbols,
            "count": symbols.len(),
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
        if let Some(root) = provider.single_root() {
            let full_path = root.join(&sym.file_path);
            std::fs::read_to_string(&full_path).ok()
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
    use std::sync::{Arc, RwLock};

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
