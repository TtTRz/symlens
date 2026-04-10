use crate::model::symbol::*;
use crate::parser::traits::{IdentifierRef, LanguageParser, RefKind};
use std::path::Path;

pub struct PythonParser;

impl LanguageParser for PythonParser {
    fn extensions(&self) -> &[&str] {
        &["py"]
    }

    fn language_name(&self) -> &str {
        "python"
    }

    fn extract_symbols(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<Symbol>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_python::LANGUAGE.into())?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse {}", file_path.display()))?;

        let mut symbols = Vec::new();
        let file_str = file_path.to_string_lossy();
        extract_py_node(tree.root_node(), source, &file_str, file_path, None, &mut symbols);
        Ok(symbols)
    }

    fn find_identifiers(&self, source: &[u8], target_name: &str) -> anyhow::Result<Vec<IdentifierRef>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_python::LANGUAGE.into())?;
        let tree = parser.parse(source, None).ok_or_else(|| anyhow::anyhow!("parse failed"))?;

        let mut refs = Vec::new();
        let lines: Vec<&str> = std::str::from_utf8(source).unwrap_or("").lines().collect();
        collect_py_identifiers(tree.root_node(), source, target_name, &lines, &mut refs);
        Ok(refs)
    }
}

fn extract_py_node(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    parent_name: Option<&str>,
    symbols: &mut Vec<Symbol>,
) {
    match node.kind() {
        "function_definition" => {
            if let Some(sym) = extract_py_function(node, source, file_str, file_path, parent_name) {
                symbols.push(sym);
            }
        }
        "class_definition" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Some(name) = node_text(name_node, source) {
                    let doc = extract_py_docstring(node, source);
                    symbols.push(Symbol {
                        id: SymbolId::new(file_str, &name, &SymbolKind::Class),
                        name: name.clone(),
                        qualified_name: name.clone(),
                        kind: SymbolKind::Class,
                        file_path: file_path.to_path_buf(),
                        span: node_span(node),
                        signature: None,
                        doc_comment: doc,
                        visibility: Visibility::Public,
                        parent: None,
                        children: vec![],
                    });

                    // Extract methods
                    if let Some(body) = node.child_by_field_name("body") {
                        let cursor = &mut body.walk();
                        for child in body.children(cursor) {
                            extract_py_node(child, source, file_str, file_path, Some(&name), symbols);
                        }
                    }
                    return; // Already handled children
                }
            }
        }
        "decorated_definition" => {
            // Recurse into the decorated item
            let cursor = &mut node.walk();
            for child in node.children(cursor) {
                if child.kind() != "decorator" {
                    extract_py_node(child, source, file_str, file_path, parent_name, symbols);
                }
            }
            return;
        }
        _ => {}
    }

    // Recurse into module-level statements
    if node.kind() == "module" || node.kind() == "block" {
        let cursor = &mut node.walk();
        for child in node.children(cursor) {
            extract_py_node(child, source, file_str, file_path, parent_name, symbols);
        }
    }
}

fn extract_py_function(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    parent_name: Option<&str>,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    let doc = extract_py_docstring(node, source);

    let (qualified, kind) = match parent_name {
        Some(parent) => (format!("{}::{}", parent, name), SymbolKind::Method),
        None => (name.clone(), SymbolKind::Function),
    };

    // Extract signature (def line)
    let sig = {
        let start = node.start_byte();
        let end = node.child_by_field_name("body")
            .map(|b| b.start_byte())
            .unwrap_or(node.end_byte());
        let sig_bytes = &source[start..end];
        String::from_utf8_lossy(sig_bytes)
            .trim()
            .trim_end_matches(':')
            .trim()
            .to_string()
    };

    Some(Symbol {
        id: SymbolId::new(file_str, &qualified, &kind),
        name,
        qualified_name: qualified,
        kind,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(sig),
        doc_comment: doc,
        visibility: Visibility::Public,
        parent: parent_name.map(|p| SymbolId::new(file_str, p, &SymbolKind::Class)),
        children: vec![],
    })
}

fn extract_py_docstring(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    // Python docstring: first statement in body is a string expression
    let body = node.child_by_field_name("body")?;
    let first_child = body.child(0)?;

    if first_child.kind() == "expression_statement" {
        let expr = first_child.child(0)?;
        if expr.kind() == "string" || expr.kind() == "concatenated_string" {
            let text = node_text(expr, source)?;
            let cleaned = text
                .trim_matches('"')
                .trim_matches('\'')
                .trim()
                .to_string();
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }
    None
}

fn collect_py_identifiers(
    node: tree_sitter::Node,
    source: &[u8],
    target_name: &str,
    lines: &[&str],
    refs: &mut Vec<IdentifierRef>,
) {
    match node.kind() {
        "comment" | "string" | "concatenated_string" => return,
        _ => {}
    }

    if node.kind() == "identifier" && node_text(node, source).as_deref() == Some(target_name) {
        let line = node.start_position().row as u32 + 1;
        let context = lines.get(line as usize - 1).unwrap_or(&"").trim().to_string();

        let kind = if let Some(parent) = node.parent() {
            match parent.kind() {
                "call" => RefKind::Call,
                "import_from_statement" | "import_statement" => RefKind::Import,
                "type" | "annotation" => RefKind::TypeRef,
                "function_definition" | "class_definition" => {
                    if parent.child_by_field_name("name").map(|n| n.id()) == Some(node.id()) {
                        RefKind::Definition
                    } else {
                        RefKind::Unknown
                    }
                }
                "attribute" => RefKind::FieldAccess,
                _ => RefKind::Unknown,
            }
        } else {
            RefKind::Unknown
        };

        refs.push(IdentifierRef { name: target_name.to_string(), line, context, kind });
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_py_identifiers(child, source, target_name, lines, refs);
    }
}

fn node_text(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source).ok().map(|s| s.to_string())
}

fn node_span(node: tree_sitter::Node) -> Span {
    let start = node.start_position();
    let end = node.end_position();
    Span {
        start_line: start.row as u32 + 1,
        end_line: end.row as u32 + 1,
        start_col: start.column as u32,
        end_col: end.column as u32,
    }
}
