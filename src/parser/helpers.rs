use crate::model::symbol::Span;
use std::path::Path;

/// Parse source code with a tree-sitter language grammar.
/// Consolidates the repeated Parser::new() + set_language() + parse()
/// boilerplate across all language parsers into a single call site.
pub fn parse_source(
    language: tree_sitter::Language,
    source: &[u8],
    file_path: &Path,
) -> anyhow::Result<tree_sitter::Tree> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language)?;
    parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse {}", file_path.display()))
}

/// Extract UTF-8 text content from a tree-sitter AST node.
pub fn node_text(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source).ok().map(|s| s.to_string())
}

/// Convert a tree-sitter node position to a 1-indexed source Span.
pub fn node_span(node: tree_sitter::Node) -> Span {
    let start = node.start_position();
    let end = node.end_position();
    Span {
        start_line: start.row as u32 + 1,
        end_line: end.row as u32 + 1,
        start_col: start.column as u32,
        end_col: end.column as u32,
    }
}

/// Extract only the first trimmed line of a node's text content.
/// Useful for multi-line declarations where only the signature line is needed.
pub fn node_text_first_line(node: tree_sitter::Node, source: &[u8]) -> String {
    node.utf8_text(source)
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}
