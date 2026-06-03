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

/// Compare a node's text against a target string without allocating.
pub fn node_text_eq(node: tree_sitter::Node, source: &[u8], target: &str) -> bool {
    node.utf8_text(source).map_or(false, |s| s == target)
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

/// Find the first direct child of `node` whose kind matches `kind`.
pub fn find_child_by_kind<'a>(
    node: tree_sitter::Node<'a>,
    kind: &str,
) -> Option<tree_sitter::Node<'a>> {
    let cursor = &mut node.walk();
    node.children(cursor).find(|&child| child.kind() == kind)
}

/// Find the last direct child of `node` whose kind matches `kind`.
pub fn last_child_by_kind<'a>(
    node: tree_sitter::Node<'a>,
    kind: &str,
) -> Option<tree_sitter::Node<'a>> {
    let cursor = &mut node.walk();
    let mut last = None;
    for child in node.children(cursor) {
        if child.kind() == kind {
            last = Some(child);
        }
    }
    last
}

/// Find the first direct child of `node` matching `kind` and return its text.
pub fn find_child_text_by_kind(
    node: tree_sitter::Node,
    kind: &str,
    source: &[u8],
) -> Option<String> {
    find_child_by_kind(node, kind).and_then(|n| node_text(n, source))
}

/// Extract a function/declaration signature by slicing source from `node.start_byte()`
/// up to the first child whose kind is in `body_kinds` (or `node.end_byte()` if none found).
/// Lines are trimmed and joined with a single space.
pub fn extract_signature(
    node: tree_sitter::Node,
    source: &[u8],
    body_kinds: &[&str],
) -> String {
    let start = node.start_byte();
    let mut end = node.end_byte();
    for kind in body_kinds {
        if let Some(body) = find_child_by_kind(node, kind) {
            end = body.start_byte();
            break;
        }
    }
    let sig = &source[start..end];
    String::from_utf8_lossy(sig)
        .trim()
        .lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract documentation comments preceding `node`.
///
/// Walks preceding siblings, collecting:
/// - Line comments matching `line_comment_kind` whose text starts with `line_prefix`
///   (the prefix is stripped).
/// - Block comments matching `block_comment_kind` that start with `block_prefix`
///   (e.g. `/**`); inner `*` markers are stripped.
///
/// Non-matching nodes terminate the walk.
pub fn extract_doc_comment(
    node: tree_sitter::Node,
    source: &[u8],
    line_comment_kind: &str,
    line_prefix: &str,
    block_comment_kind: &str,
    block_prefix: &str,
) -> Option<String> {
    let mut comments = Vec::new();
    let mut sibling = node.prev_sibling();
    while let Some(s) = sibling {
        if s.kind() == line_comment_kind {
            let text = node_text(s, source)?;
            if text.starts_with(line_prefix) {
                let cleaned = text.trim_start_matches(line_prefix).trim();
                comments.push(cleaned.to_string());
            } else {
                break;
            }
        } else if s.kind() == block_comment_kind {
            if let Some(text) = node_text(s, source) {
                if text.starts_with(block_prefix) {
                    let cleaned = text
                        .trim_start_matches(block_prefix)
                        .trim_end_matches("*/")
                        .lines()
                        .map(|l| l.trim().trim_start_matches('*').trim())
                        .filter(|l| !l.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n");
                    comments.push(cleaned);
                } else {
                    break;
                }
            }
        } else {
            break;
        }
        sibling = s.prev_sibling();
    }
    if comments.is_empty() {
        None
    } else {
        comments.reverse();
        Some(comments.join("\n"))
    }
}
