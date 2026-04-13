use crate::model::symbol::*;
use crate::parser::traits::{CallEdge, IdentifierRef, ImportInfo, LanguageParser, RefKind};
use std::path::Path;

pub struct TypeScriptParser;

impl LanguageParser for TypeScriptParser {
    fn extensions(&self) -> &[&str] {
        &["ts", "tsx"]
    }

    fn extract_symbols(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<Symbol>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse {}", file_path.display()))?;

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
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse {}", file_path.display()))?;
        let mut edges = Vec::new();
        collect_ts_calls(tree.root_node(), source, None, &mut edges);
        Ok(edges)
    }

    fn find_identifiers(
        &self,
        source: &[u8],
        target_name: &str,
    ) -> anyhow::Result<Vec<IdentifierRef>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("parse failed"))?;
        let mut refs = Vec::new();
        let lines: Vec<&str> = std::str::from_utf8(source).unwrap_or("").lines().collect();
        collect_ts_identifiers(tree.root_node(), source, target_name, &lines, &mut refs);
        Ok(refs)
    }

    fn extract_imports(&self, source: &[u8], _file_path: &Path) -> anyhow::Result<Vec<ImportInfo>> {
        let source_str = std::str::from_utf8(source).unwrap_or("");
        let mut imports = Vec::new();
        for line in source_str.lines() {
            let trimmed = line.trim();
            // import { Foo, Bar } from './module'
            if trimmed.starts_with("import ")
                && let Some(from_pos) = trimmed.find(" from ")
            {
                let names_part = &trimmed[7..from_pos]; // between "import " and " from "
                let module = trimmed[from_pos + 6..]
                    .trim()
                    .trim_matches(|c| c == '\'' || c == '"' || c == ';');
                let names: Vec<String> = names_part
                    .replace(['{', '}'], "")
                    .split(',')
                    .map(|n| {
                        n.trim()
                            .split(" as ")
                            .next()
                            .unwrap_or("")
                            .trim()
                            .to_string()
                    })
                    .filter(|n| !n.is_empty() && n != "*")
                    .collect();
                if !names.is_empty() {
                    imports.push(ImportInfo {
                        module_path: module.to_string(),
                        names,
                    });
                }
            }
        }
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
        signature: Some(extract_first_line(node, source)),
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
                signature: Some(extract_first_line(node, source)),
                doc_comment: extract_ts_doc(node, source),
                visibility: vis,
                parent: None,
                children: vec![],
            });
        }
    }
}

// ─── Utility helpers ────────────────────────────────────────────────

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

fn extract_first_line(node: tree_sitter::Node, source: &[u8]) -> String {
    node.utf8_text(source)
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

// ─── Call extraction ────────────────────────────────────────────────

fn collect_ts_calls(
    node: tree_sitter::Node,
    source: &[u8],
    current_fn: Option<&str>,
    edges: &mut Vec<CallEdge>,
) {
    let mut fn_name = current_fn;
    if (node.kind() == "function_declaration" || node.kind() == "method_definition")
        && let Some(name_node) = node.child_by_field_name("name")
        && let Some(name) = node_text(name_node, source)
    {
        fn_name = Some(Box::leak(name.into_boxed_str()));
    }

    if node.kind() == "call_expression"
        && let Some(caller) = fn_name
        && let Some(func_node) = node.child_by_field_name("function")
        && let Some(callee) = node_text(func_node, source)
    {
        // Clean up: "obj.method" → "method"
        let clean = callee.rsplit('.').next().unwrap_or(&callee).to_string();
        edges.push((caller.to_string(), clean));
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_ts_calls(child, source, fn_name, edges);
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
