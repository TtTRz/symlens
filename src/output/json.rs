use crate::commands::IndexProvider;
use crate::model::symbol::Symbol;
use serde_json;

/// Format search results as a JSON array of symbol objects.
/// Shared between CLI and MCP — MCP wraps the result with `{ "results": ..., "count": N }`.
pub fn format_search_results(symbols: &[(&Symbol, f32)]) -> Vec<serde_json::Value> {
    symbols
        .iter()
        .map(|(sym, score)| {
            serde_json::json!({
                "id": sym.id.0,
                "name": sym.name,
                "qualified_name": sym.qualified_name,
                "kind": sym.kind.as_str(),
                "file": sym.file_path.to_string_lossy(),
                "lines": [sym.span.start_line, sym.span.end_line],
                "signature": sym.signature,
                "doc": sym.doc_comment,
                "score": score,
            })
        })
        .collect()
}

/// Format a list of symbols as a JSON array string (CLI `--json` mode).
pub fn format_symbols(symbols: &[(&Symbol, f32)]) -> String {
    let items = format_search_results(symbols);
    serde_json::to_string_pretty(&items).unwrap_or_default()
}

/// Format a single symbol as JSON.
pub fn format_symbol(sym: &Symbol, source: Option<&str>) -> String {
    let val = format_symbol_value(sym, source);
    serde_json::to_string_pretty(&val).unwrap_or_default()
}

/// Build a JSON value for a single symbol.
/// Shared between CLI and MCP — ensures consistent field names.
pub fn format_symbol_value(sym: &Symbol, source: Option<&str>) -> serde_json::Value {
    let mut val = serde_json::json!({
        "id": sym.id.0,
        "name": sym.name,
        "qualified_name": sym.qualified_name,
        "kind": sym.kind.as_str(),
        "file": sym.file_path.to_string_lossy(),
        "lines": [sym.span.start_line, sym.span.end_line],
        "signature": sym.signature,
        "doc": sym.doc_comment,
        "visibility": format!("{:?}", sym.visibility),
        "parent": sym.parent.as_ref().map(|p| &p.0),
    });

    if let Some(src) = source {
        val["source"] = serde_json::Value::String(src.to_string());
    }

    val
}

/// Enrich a caller/callee name with symbol metadata from the index.
/// Returns `{name, file, line, kind, signature}` if symbol found, else `{name}`.
pub fn enrich_caller_json(name: &str, provider: &IndexProvider) -> serde_json::Value {
    if let Some(sym) = provider.find_symbol(name) {
        serde_json::json!({
            "name": name,
            "file": sym.file_path.to_string_lossy(),
            "line": sym.span.start_line,
            "kind": sym.kind.as_str(),
            "signature": sym.signature,
        })
    } else {
        serde_json::json!({ "name": name })
    }
}

/// Enrich a list of caller/callee names, truncating to limit.
pub fn enrich_callers_json(
    names: &[&str],
    limit: usize,
    provider: &IndexProvider,
) -> Vec<serde_json::Value> {
    names
        .iter()
        .take(limit)
        .map(|n| enrich_caller_json(n, provider))
        .collect()
}
