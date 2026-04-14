use crate::model::symbol::*;
use crate::parser::traits::{CallEdge, IdentifierRef, ImportInfo, LanguageParser, RefKind};
use std::path::Path;

use super::helpers::{node_span, node_text, node_text_first_line, parse_source};

pub struct TypeScriptParser;

impl LanguageParser for TypeScriptParser {
    fn extensions(&self) -> &[&str] {
        &["ts", "tsx", "js", "jsx"]
    }

    fn language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn extract_symbols(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<Symbol>> {
        let tree = parse_source(self.language(), source, file_path)?;

        let mut symbols = Vec::new();
        let file_str = file_path.to_string_lossy();
        extract_ts_node(
            tree.root_node(),
            source,
            &file_str,
            file_path,
            None,
            &mut symbols,
        );
        Ok(symbols)
    }

    fn extract_calls(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<CallEdge>> {
        let tree = parse_source(self.language(), source, file_path)?;
        let mut edges = Vec::new();
        collect_ts_calls(tree.root_node(), source, None, &mut edges);
        Ok(edges)
    }

    fn find_identifiers(
        &self,
        source: &[u8],
        target_name: &str,
    ) -> anyhow::Result<Vec<IdentifierRef>> {
        let tree = parse_source(self.language(), source, Path::new(""))?;
        let mut refs = Vec::new();
        let lines: Vec<&str> = std::str::from_utf8(source).unwrap_or("").lines().collect();
        collect_ts_identifiers(tree.root_node(), source, target_name, &lines, &mut refs);
        Ok(refs)
    }

    fn extract_imports(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<ImportInfo>> {
        let tree = parse_source(self.language(), source, file_path)?;

        let mut imports = Vec::new();
        collect_ts_imports(tree.root_node(), source, &mut imports);
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
        extract_ts_node(
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
        collect_ts_calls(tree.root_node(), source, None, &mut edges);
        Ok(edges)
    }

    fn extract_imports_from_tree(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        _file_path: &Path,
    ) -> anyhow::Result<Vec<ImportInfo>> {
        let mut imports = Vec::new();
        collect_ts_imports(tree.root_node(), source, &mut imports);
        Ok(imports)
    }
}

fn extract_ts_node(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    parent_name: Option<&str>,
    symbols: &mut Vec<Symbol>,
) {
    match node.kind() {
        "function_declaration" => {
            if let Some(sym) = extract_ts_function(node, source, file_str, file_path, parent_name) {
                symbols.push(sym);
            }
        }
        "class_declaration" => {
            if let Some(sym) = extract_ts_class(node, source, file_str, file_path) {
                let class_name = sym.name.clone();
                symbols.push(sym);
                // Extract class body
                if let Some(body) = node.child_by_field_name("body") {
                    extract_ts_class_body(body, source, file_str, file_path, &class_name, symbols);
                }
            }
        }
        "interface_declaration" => {
            if let Some(sym) = extract_ts_interface(node, source, file_str, file_path) {
                symbols.push(sym);
            }
        }
        "type_alias_declaration" => {
            if let Some(sym) = extract_ts_type_alias(node, source, file_str, file_path) {
                symbols.push(sym);
            }
        }
        "enum_declaration" => {
            if let Some(sym) = extract_ts_enum(node, source, file_str, file_path) {
                symbols.push(sym);
            }
        }
        "lexical_declaration" | "variable_declaration" => {
            extract_ts_variable(node, source, file_str, file_path, symbols);
        }
        "export_statement" => {
            // Recurse into export statements to find the declaration inside
            let cursor = &mut node.walk();
            for child in node.children(cursor) {
                extract_ts_node(child, source, file_str, file_path, parent_name, symbols);
            }
            return; // Already recursed
        }
        _ => {}
    }

    // Recurse into top-level children
    if node.kind() == "program" || node.kind() == "statement_block" {
        let cursor = &mut node.walk();
        for child in node.children(cursor) {
            extract_ts_node(child, source, file_str, file_path, parent_name, symbols);
        }
    }
}

fn extract_ts_function(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    parent_name: Option<&str>,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    let doc = extract_ts_doc(node, source);

    let (qualified, kind) = match parent_name {
        Some(parent) => (format!("{}::{}", parent, name), SymbolKind::Method),
        None => (name.clone(), SymbolKind::Function),
    };

    let vis = if has_export(node) {
        Visibility::Public
    } else {
        Visibility::Private
    };

    Some(Symbol {
        id: SymbolId::new(file_str, &qualified, &kind),
        name,
        qualified_name: qualified,
        kind,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(extract_ts_signature(node, source)),
        doc_comment: doc,
        visibility: vis,
        parent: parent_name.map(|p| SymbolId::new(file_str, p, &SymbolKind::Class)),
        children: vec![],
    })
}

fn extract_ts_class(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    let vis = if has_export(node) {
        Visibility::Public
    } else {
        Visibility::Private
    };

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Class),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Class,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: None,
        doc_comment: extract_ts_doc(node, source),
        visibility: vis,
        parent: None,
        children: vec![],
    })
}

fn extract_ts_class_body(
    body: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    class_name: &str,
    symbols: &mut Vec<Symbol>,
) {
    let cursor = &mut body.walk();
    for child in body.children(cursor) {
        match child.kind() {
            "method_definition" => {
                if let Some(name_node) = child.child_by_field_name("name")
                    && let Some(name) = node_text(name_node, source)
                {
                    let qualified = format!("{}::{}", class_name, name);
                    symbols.push(Symbol {
                        id: SymbolId::new(file_str, &qualified, &SymbolKind::Method),
                        name,
                        qualified_name: qualified,
                        kind: SymbolKind::Method,
                        file_path: file_path.to_path_buf(),
                        span: node_span(child),
                        signature: Some(extract_ts_signature(child, source)),
                        doc_comment: extract_ts_doc(child, source),
                        visibility: Visibility::Public,
                        parent: Some(SymbolId::new(file_str, class_name, &SymbolKind::Class)),
                        children: vec![],
                    });
                }
            }
            "public_field_definition" | "property_definition" => {
                if let Some(name_node) = child.child_by_field_name("name")
                    && let Some(name) = node_text(name_node, source)
                {
                    let qualified = format!("{}::{}", class_name, name);
                    symbols.push(Symbol {
                        id: SymbolId::new(file_str, &qualified, &SymbolKind::Field),
                        name,
                        qualified_name: qualified,
                        kind: SymbolKind::Field,
                        file_path: file_path.to_path_buf(),
                        span: node_span(child),
                        signature: None,
                        doc_comment: None,
                        visibility: Visibility::Public,
                        parent: Some(SymbolId::new(file_str, class_name, &SymbolKind::Class)),
                        children: vec![],
                    });
                }
            }
            _ => {}
        }
    }
}

fn extract_ts_interface(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Interface),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Interface,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: None,
        doc_comment: extract_ts_doc(node, source),
        visibility: if has_export(node) {
            Visibility::Public
        } else {
            Visibility::Private
        },
        parent: None,
        children: vec![],
    })
}

fn extract_ts_type_alias(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::TypeAlias),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::TypeAlias,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(node_text_first_line(node, source)),
        doc_comment: extract_ts_doc(node, source),
        visibility: if has_export(node) {
            Visibility::Public
        } else {
            Visibility::Private
        },
        parent: None,
        children: vec![],
    })
}

fn extract_ts_enum(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Enum),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Enum,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: None,
        doc_comment: extract_ts_doc(node, source),
        visibility: if has_export(node) {
            Visibility::Public
        } else {
            Visibility::Private
        },
        parent: None,
        children: vec![],
    })
}

fn extract_ts_variable(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    symbols: &mut Vec<Symbol>,
) {
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        if child.kind() == "variable_declarator"
            && let Some(name_node) = child.child_by_field_name("name")
            && let Some(name) = node_text(name_node, source)
        {
            // Check if value is an arrow function or function
            let is_function = child
                .child_by_field_name("value")
                .map(|v| v.kind() == "arrow_function" || v.kind() == "function")
                .unwrap_or(false);

            let kind = if is_function {
                SymbolKind::Function
            } else {
                SymbolKind::Constant
            };

            let vis = if has_export(node) {
                Visibility::Public
            } else {
                Visibility::Private
            };

            symbols.push(Symbol {
                id: SymbolId::new(file_str, &name, &kind),
                name: name.clone(),
                qualified_name: name,
                kind,
                file_path: file_path.to_path_buf(),
                span: node_span(node),
                signature: Some(node_text_first_line(node, source)),
                doc_comment: extract_ts_doc(node, source),
                visibility: vis,
                parent: None,
                children: vec![],
            });
        }
    }
}

// ─── Import extraction ─────────────────────────────────────────────

/// Extract import statements from TypeScript AST.
/// Handles: `import { A, B } from 'module'`, `import X from 'module'`,
/// `import * as N from 'module'`, `import 'side-effect'`,
/// `import type { T } from 'module'`, `export { X } from 'module'`.
fn collect_ts_imports(node: tree_sitter::Node, source: &[u8], imports: &mut Vec<ImportInfo>) {
    match node.kind() {
        "import_statement" => {
            // import_clause children: import_clause, string_literal (source), semicolon
            let mut module = None;
            let mut names = Vec::new();

            let cursor = &mut node.walk();
            for child in node.children(cursor) {
                match child.kind() {
                    "import_clause" => {
                        // import_clause contains: named_imports, namespace_import, identifier
                        let cc = &mut child.walk();
                        for sub in child.children(cc) {
                            match sub.kind() {
                                "named_imports" => {
                                    // { A, B as C, type D }
                                    let nc = &mut sub.walk();
                                    for spec in sub.children(nc) {
                                        if spec.kind() == "import_specifier" {
                                            // import_specifier has "name" and optionally "alias"
                                            if let Some(name_node) =
                                                spec.child_by_field_name("name")
                                                && let Some(name) = node_text(name_node, source)
                                            {
                                                names.push(name);
                                            }
                                        }
                                    }
                                }
                                "namespace_import" => {
                                    // import * as Foo
                                    if let Some(alias_node) = child_by_field_name(&sub, "alias")
                                        && let Some(alias) = node_text(alias_node, source)
                                    {
                                        names.push(alias);
                                    }
                                }
                                "identifier" => {
                                    // import Foo from 'module' (default import)
                                    if let Some(name) = node_text(sub, source) {
                                        names.push(name);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    "string" | "string_literal" => {
                        // Module source: 'path' or "path"
                        let text = node_text(child, source).unwrap_or_default();
                        module = Some(
                            text.trim_matches(|c| c == '\'' || c == '"' || c == '`')
                                .to_string(),
                        );
                    }
                    _ => {}
                }
            }

            if let Some(module_path) = module
                && !names.is_empty()
            {
                imports.push(ImportInfo { module_path, names });
            }
        }
        "export_statement" => {
            // export { X } from 'module' — re-exports
            let mut module = None;
            let mut names = Vec::new();

            let cursor = &mut node.walk();
            for child in node.children(cursor) {
                match child.kind() {
                    "export_clause" => {
                        let ec = &mut child.walk();
                        for spec in child.children(ec) {
                            if spec.kind() == "export_specifier"
                                && let Some(name_node) = spec.child_by_field_name("name")
                                && let Some(name) = node_text(name_node, source)
                            {
                                names.push(name);
                            }
                        }
                    }
                    "string" | "string_literal" => {
                        let text = node_text(child, source).unwrap_or_default();
                        module = Some(
                            text.trim_matches(|c| c == '\'' || c == '"' || c == '`')
                                .to_string(),
                        );
                    }
                    _ => {}
                }
            }

            if let Some(module_path) = module
                && !names.is_empty()
            {
                imports.push(ImportInfo { module_path, names });
            }
        }
        _ => {}
    }

    // Recurse into children
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_ts_imports(child, source, imports);
    }
}

/// Helper: get child by field name for any node (workaround for tree-sitter API).
fn child_by_field_name<'a>(
    node: &tree_sitter::Node<'a>,
    field: &str,
) -> Option<tree_sitter::Node<'a>> {
    node.child_by_field_name(field)
}

fn has_export(node: tree_sitter::Node) -> bool {
    node.parent()
        .map(|p| p.kind() == "export_statement")
        .unwrap_or(false)
}

fn extract_ts_doc(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut sibling = node.prev_sibling();
    // Also check parent's prev sibling for exported items
    if sibling.is_none()
        && let Some(parent) = node.parent()
        && parent.kind() == "export_statement"
    {
        sibling = parent.prev_sibling();
    }

    if let Some(s) = sibling
        && s.kind() == "comment"
    {
        let text = node_text(s, source)?;
        if text.starts_with("/**") {
            // Parse JSDoc comment
            let cleaned: String = text
                .trim_start_matches("/**")
                .trim_end_matches("*/")
                .lines()
                .map(|l| l.trim().trim_start_matches('*').trim())
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }
    None
}

fn extract_ts_signature(node: tree_sitter::Node, source: &[u8]) -> String {
    let start = node.start_byte();
    let mut end = node.end_byte();

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        if child.kind() == "statement_block" {
            end = child.start_byte();
            break;
        }
    }

    let sig_bytes = &source[start..end];
    String::from_utf8_lossy(sig_bytes)
        .trim()
        .lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join(" ")
}

// ─── Call extraction ────────────────────────────────────────────────

fn collect_ts_calls(
    node: tree_sitter::Node,
    source: &[u8],
    current_fn: Option<&str>,
    edges: &mut Vec<CallEdge>,
) {
    let mut fn_name: Option<String> = current_fn.map(|s| s.to_string());
    if (node.kind() == "function_declaration" || node.kind() == "method_definition")
        && let Some(name_node) = node.child_by_field_name("name")
        && let Some(name) = node_text(name_node, source)
    {
        fn_name = Some(name);
    }

    if node.kind() == "call_expression"
        && let Some(ref caller) = fn_name
        && let Some(func_node) = node.child_by_field_name("function")
        && let Some(callee) = node_text(func_node, source)
    {
        // Clean up: "obj.method" → "method"
        let clean = callee.rsplit('.').next().unwrap_or(&callee).to_string();
        edges.push((caller.clone(), clean));
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_ts_calls(child, source, fn_name.as_deref(), edges);
    }
}

// ─── Identifier references ──────────────────────────────────────────

fn collect_ts_identifiers(
    node: tree_sitter::Node,
    source: &[u8],
    target: &str,
    lines: &[&str],
    refs: &mut Vec<IdentifierRef>,
) {
    match node.kind() {
        "comment" | "string" | "template_string" | "template_substitution" => return,
        _ => {}
    }

    if node.kind() == "identifier" && node_text(node, source).as_deref() == Some(target) {
        let line = node.start_position().row as u32 + 1;
        let context = lines
            .get(line as usize - 1)
            .unwrap_or(&"")
            .trim()
            .to_string();
        let kind = if let Some(parent) = node.parent() {
            match parent.kind() {
                "call_expression" => RefKind::Call,
                "import_specifier" | "import_clause" | "import_statement" => RefKind::Import,
                "type_annotation" | "type_identifier" | "generic_type" => RefKind::TypeRef,
                "function_declaration" | "class_declaration" | "method_definition" => {
                    if parent.child_by_field_name("name").map(|n| n.id()) == Some(node.id()) {
                        RefKind::Definition
                    } else {
                        RefKind::Unknown
                    }
                }
                "member_expression" => {
                    if let Some(gp) = parent.parent() {
                        if gp.kind() == "call_expression" {
                            RefKind::Call
                        } else {
                            RefKind::FieldAccess
                        }
                    } else {
                        RefKind::FieldAccess
                    }
                }
                "new_expression" => RefKind::Constructor,
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
        collect_ts_identifiers(child, source, target, lines, refs);
    }
}
