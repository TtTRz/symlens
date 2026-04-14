//! WASM API surface for symlens.
//!
//! Provides JavaScript-callable functions for parsing source code,
//! extracting symbols and call edges, building call graphs, and querying them.

use wasm_bindgen::prelude::*;

use crate::graph::call_graph::CallGraph;
use crate::parser::registry::LanguageRegistry;
use std::path::Path;

/// Parse source code and return symbols as JSON.
///
/// # Arguments
/// * `filename` - e.g. "main.rs", "app.ts" (used to detect language)
/// * `source` - the source code text
///
/// # Returns
/// JSON array of symbol objects
#[wasm_bindgen]
pub fn parse_source(filename: &str, source: &str) -> Result<JsValue, JsError> {
    let registry = LanguageRegistry::new();
    let path = Path::new(filename);

    let parser = registry
        .parser_for(path)
        .ok_or_else(|| JsError::new(&format!("Unsupported file type: {filename}")))?;

    let symbols = parser
        .extract_symbols(source.as_bytes(), path)
        .map_err(|e| JsError::new(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&symbols).map_err(|e| JsError::new(&e.to_string()))
}

/// Extract call edges from source code.
///
/// # Returns
/// JSON array of `[caller_name, callee_name]` pairs
#[wasm_bindgen]
pub fn extract_calls(filename: &str, source: &str) -> Result<JsValue, JsError> {
    let registry = LanguageRegistry::new();
    let path = Path::new(filename);

    let parser = registry
        .parser_for(path)
        .ok_or_else(|| JsError::new(&format!("Unsupported file type: {filename}")))?;

    let edges = parser
        .extract_calls(source.as_bytes(), path)
        .map_err(|e| JsError::new(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&edges).map_err(|e| JsError::new(&e.to_string()))
}

/// Extract import statements from source code.
///
/// # Returns
/// JSON array of import info objects
#[wasm_bindgen]
pub fn extract_imports(filename: &str, source: &str) -> Result<JsValue, JsError> {
    let registry = LanguageRegistry::new();
    let path = Path::new(filename);

    let parser = registry
        .parser_for(path)
        .ok_or_else(|| JsError::new(&format!("Unsupported file type: {filename}")))?;

    let imports = parser
        .extract_imports(source.as_bytes(), path)
        .map_err(|e| JsError::new(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&imports).map_err(|e| JsError::new(&e.to_string()))
}

/// Build a call graph from edges JSON.
///
/// # Arguments
/// * `edges_json` - JSON array of `[caller, callee]` string pairs
///
/// # Returns
/// JSON: `{ "nodes": [...], "edges": [[from, to], ...] }`
#[wasm_bindgen]
pub fn build_call_graph(edges_json: JsValue) -> Result<JsValue, JsError> {
    let edges: Vec<(String, String)> =
        serde_wasm_bindgen::from_value(edges_json).map_err(|e| JsError::new(&e.to_string()))?;

    let graph = CallGraph::build(&edges);

    serde_wasm_bindgen::to_value(&graph).map_err(|e| JsError::new(&e.to_string()))
}

/// Query direct callers of a symbol from a serialized call graph.
///
/// # Arguments
/// * `graph_json` - CallGraph JSON (from `build_call_graph`)
/// * `symbol_name` - the symbol to query
///
/// # Returns
/// JSON array of caller name strings
#[wasm_bindgen]
pub fn query_callers(graph_json: JsValue, symbol_name: &str) -> Result<JsValue, JsError> {
    let mut graph: CallGraph =
        serde_wasm_bindgen::from_value(graph_json).map_err(|e| JsError::new(&e.to_string()))?;
    graph.rebuild_index();

    let result: Vec<&str> = graph.callers(symbol_name);
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsError::new(&e.to_string()))
}

/// Query direct callees of a symbol from a serialized call graph.
///
/// # Arguments
/// * `graph_json` - CallGraph JSON (from `build_call_graph`)
/// * `symbol_name` - the symbol to query
///
/// # Returns
/// JSON array of callee name strings
#[wasm_bindgen]
pub fn query_callees(graph_json: JsValue, symbol_name: &str) -> Result<JsValue, JsError> {
    let mut graph: CallGraph =
        serde_wasm_bindgen::from_value(graph_json).map_err(|e| JsError::new(&e.to_string()))?;
    graph.rebuild_index();

    let result: Vec<&str> = graph.callees(symbol_name);
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsError::new(&e.to_string()))
}

/// Get list of supported file extensions.
///
/// # Returns
/// JSON array of extension strings
#[wasm_bindgen]
pub fn supported_extensions() -> Result<JsValue, JsError> {
    let extensions = vec![
        "rs", "ts", "tsx", "py", "swift", "go", "dart", "c", "h", "cpp", "cc", "cxx", "hpp", "hh",
        "kt", "kts",
    ];
    serde_wasm_bindgen::to_value(&extensions).map_err(|e| JsError::new(&e.to_string()))
}
