use crate::model::symbol::Symbol;
use serde_json;

/// Format a list of symbols as JSON.
pub fn format_symbols(symbols: &[(&Symbol, f32)]) -> String {
    let items: Vec<serde_json::Value> = symbols
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
        .collect();

    serde_json::to_string_pretty(&items).unwrap_or_default()
}

/// Format a single symbol as JSON.
pub fn format_symbol(sym: &Symbol, source: Option<&str>) -> String {
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

    serde_json::to_string_pretty(&val).unwrap_or_default()
}
