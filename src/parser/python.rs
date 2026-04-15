use crate::model::symbol::*;
use crate::parser::traits::{CallEdge, IdentifierRef, ImportInfo, LanguageParser, RefKind};
use std::path::Path;

use super::helpers::{node_span, node_text, parse_source};

pub struct PythonParser;

impl LanguageParser for PythonParser {
    fn extensions(&self) -> &[&str] {
        &["py"]
    }

    fn language(&self) -> tree_sitter::Language {
        tree_sitter_python::LANGUAGE.into()
    }

    fn extract_symbols(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<Symbol>> {
        let tree = parse_source(self.language(), source, file_path)?;

        let mut symbols = Vec::new();
        let file_str = file_path.to_string_lossy();
        extract_py_node(
            tree.root_node(),
            source,
            &file_str,
            file_path,
            None,
            &mut symbols,
        );
        Ok(symbols)
    }

    fn find_identifiers(
        &self,
        source: &[u8],
        target_name: &str,
    ) -> anyhow::Result<Vec<IdentifierRef>> {
        let tree = parse_source(self.language(), source, Path::new(""))?;

        let mut refs = Vec::new();
        let lines: Vec<&str> = std::str::from_utf8(source).unwrap_or("").lines().collect();
        collect_py_identifiers(tree.root_node(), source, target_name, &lines, &mut refs);
        Ok(refs)
    }

    fn extract_calls(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<CallEdge>> {
        let tree = parse_source(self.language(), source, file_path)?;
        let mut edges = Vec::new();
        collect_py_calls(tree.root_node(), source, None, &mut edges);
        Ok(edges)
    }

    fn extract_imports(&self, source: &[u8], _file_path: &Path) -> anyhow::Result<Vec<ImportInfo>> {
        let tree = parse_source(self.language(), source, Path::new(""))?;

        let mut imports = Vec::new();
        collect_py_imports(tree.root_node(), source, &mut imports);
        Ok(imports)
    }

    fn extract_symbols_from_tree(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        file_path: &Path,
    ) -> anyhow::Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let file_str = file_path.to_string_lossy();
        extract_py_node(
            tree.root_node(),
            source,
            &file_str,
            file_path,
            None,
            &mut symbols,
        );
        Ok(symbols)
    }

    fn extract_calls_from_tree(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        _file_path: &Path,
    ) -> anyhow::Result<Vec<CallEdge>> {
        let mut edges = Vec::new();
        collect_py_calls(tree.root_node(), source, None, &mut edges);
        Ok(edges)
    }

    fn extract_imports_from_tree(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        _file_path: &Path,
    ) -> anyhow::Result<Vec<ImportInfo>> {
        let mut imports = Vec::new();
        collect_py_imports(tree.root_node(), source, &mut imports);
        Ok(imports)
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
            if let Some(name_node) = node.child_by_field_name("name")
                && let Some(name) = node_text(name_node, source)
            {
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
        let end = node
            .child_by_field_name("body")
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
            let cleaned = text.trim_matches('"').trim_matches('\'').trim().to_string();
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
        let context = lines
            .get(line as usize - 1)
            .unwrap_or(&"")
            .trim()
            .to_string();

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

        refs.push(IdentifierRef {
            line,
            context,
            kind,
        });
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_py_identifiers(child, source, target_name, lines, refs);
    }
}

// ─── Call extraction ────────────────────────────────────────────────

fn collect_py_calls(
    node: tree_sitter::Node,
    source: &[u8],
    current_fn: Option<&str>,
    edges: &mut Vec<CallEdge>,
) {
    let mut fn_name: Option<String> = current_fn.map(|s| s.to_string());
    if node.kind() == "function_definition"
        && let Some(name_node) = node.child_by_field_name("name")
        && let Some(name) = node_text(name_node, source)
    {
        fn_name = Some(name);
    }

    if node.kind() == "call"
        && let Some(ref caller) = fn_name
        && let Some(func_node) = node.child_by_field_name("function")
        && let Some(callee) = node_text(func_node, source)
    {
        let clean = callee.rsplit('.').next().unwrap_or(&callee).to_string();
        edges.push((caller.clone(), clean));
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_py_calls(child, source, fn_name.as_deref(), edges);
    }
}

// ─── Import extraction ──────────────────────────────────────────────

/// Extract import statements from Python AST.
/// Handles: `import os`, `import os.path`, `import sys as system`,
/// `from foo.bar import Baz, Qux`, `from foo import *`,
/// `from . import something`, `from ..package import Module`.
fn collect_py_imports(node: tree_sitter::Node, source: &[u8], imports: &mut Vec<ImportInfo>) {
    match node.kind() {
        "import_statement" => {
            // import os | import os.path | import sys as system
            let mut module = String::new();
            let mut names = Vec::new();
            let cursor = &mut node.walk();
            for child in node.children(cursor) {
                if child.kind() == "dotted_name" {
                    // The full module path
                    if let Some(text) = node_text(child, source) {
                        module = text.clone();
                        // Last segment is the imported name
                        let name = text.rsplit('.').next().unwrap_or(&text).to_string();
                        names.push(name);
                    }
                } else if child.kind() == "aliased_import" {
                    // import foo as bar
                    if let Some(name_node) = child.child_by_field_name("name")
                        && let Some(name) = node_text(name_node, source)
                        && module.is_empty()
                    {
                        module = name.clone();
                    }
                    if let Some(alias_node) = child.child_by_field_name("alias")
                        && let Some(alias) = node_text(alias_node, source)
                    {
                        names.push(alias);
                    }
                }
            }
            if !names.is_empty() {
                imports.push(ImportInfo {
                    module_path: module,
                    names,
                });
            }
        }
        "import_from_statement" => {
            // from foo.bar import Baz, Qux | from . import something
            let mut module = String::new();
            let mut names = Vec::new();
            let cursor = &mut node.walk();
            for child in node.children(cursor) {
                if child.kind() == "dotted_name" {
                    if let Some(text) = node_text(child, source) {
                        // Could be the module path or an imported name
                        // If module is not set yet, this is the module;
                        // otherwise it's an imported name
                        if module.is_empty() {
                            module = text;
                        } else {
                            // from x import Y → Y is a dotted_name after "import"
                            let name = text.rsplit('.').next().unwrap_or(&text).to_string();
                            names.push(name);
                        }
                    }
                } else if child.kind() == "relative_import" {
                    // from . or from ..package
                    if let Some(text) = node_text(child, source) {
                        module = text;
                    }
                } else if child.kind() == "wildcard_import" {
                    // from foo import * — skip (no specific names)
                } else if child.kind() == "aliased_import" {
                    // from foo import Bar as Baz
                    if let Some(alias_node) = child.child_by_field_name("alias")
                        && let Some(alias) = node_text(alias_node, source)
                    {
                        names.push(alias);
                    }
                }
            }
            if !names.is_empty() && !module.is_empty() {
                imports.push(ImportInfo {
                    module_path: module,
                    names,
                });
            }
        }
        _ => {}
    }

    // Recurse into children
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_py_imports(child, source, imports);
    }
}
