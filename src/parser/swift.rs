use super::helpers::{node_span, node_text, parse_source};
use crate::model::symbol::*;
use crate::parser::traits::{CallEdge, IdentifierRef, ImportInfo, LanguageParser, RefKind};
use std::path::Path;

pub struct SwiftParser;

impl LanguageParser for SwiftParser {
    fn extensions(&self) -> &[&str] {
        &["swift"]
    }

    fn language(&self) -> tree_sitter::Language {
        tree_sitter_swift::LANGUAGE.into()
    }

    fn extract_symbols(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<Symbol>> {
        let tree = parse_source(self.language(), source, file_path)?;

        let mut symbols = Vec::new();
        let file_str = file_path.to_string_lossy();
        extract_swift_node(
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
        let tree = parse_source(self.language(), source, std::path::Path::new(""))?;

        let mut refs = Vec::new();
        let lines: Vec<&str> = std::str::from_utf8(source).unwrap_or("").lines().collect();
        collect_swift_ids(tree.root_node(), source, target_name, &lines, &mut refs);
        Ok(refs)
    }

    fn extract_calls(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<CallEdge>> {
        let tree = parse_source(self.language(), source, file_path)?;
        let mut edges = Vec::new();
        collect_swift_calls(tree.root_node(), source, None, &mut edges);
        Ok(edges)
    }

    fn extract_imports(&self, source: &[u8], _file_path: &Path) -> anyhow::Result<Vec<ImportInfo>> {
        let source_str = std::str::from_utf8(source).unwrap_or("");
        let mut imports = Vec::new();
        for line in source_str.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("import ") {
                let module = rest.trim();
                if !module.is_empty() {
                    let name = module.rsplit('.').next().unwrap_or(module).to_string();
                    imports.push(ImportInfo {
                        module_path: module.to_string(),
                        names: vec![name],
                    });
                }
            }
        }
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
        extract_swift_node(
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
        collect_swift_calls(tree.root_node(), source, None, &mut edges);
        Ok(edges)
    }

    fn extract_imports_from_tree(
        &self,
        _tree: &tree_sitter::Tree,
        source: &[u8],
        file_path: &Path,
    ) -> anyhow::Result<Vec<ImportInfo>> {
        // Still uses regex — will be migrated to tree-sitter later
        self.extract_imports(source, file_path)
    }
}

fn extract_swift_node(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    parent_name: Option<&str>,
    symbols: &mut Vec<Symbol>,
) {
    match node.kind() {
        "function_declaration" => {
            if let Some(sym) = extract_swift_func(node, source, file_str, file_path, parent_name) {
                symbols.push(sym);
            }
        }
        "class_declaration" => {
            extract_swift_type(
                node,
                source,
                file_str,
                file_path,
                SymbolKind::Class,
                symbols,
            );
        }
        "struct_declaration" => {
            extract_swift_type(
                node,
                source,
                file_str,
                file_path,
                SymbolKind::Struct,
                symbols,
            );
        }
        "enum_declaration" => {
            extract_swift_type(node, source, file_str, file_path, SymbolKind::Enum, symbols);
        }
        "protocol_declaration" => {
            extract_swift_type(
                node,
                source,
                file_str,
                file_path,
                SymbolKind::Interface,
                symbols,
            );
        }
        _ => {}
    }

    // Recurse
    let recurse_kinds = [
        "source_file",
        "statements",
        "class_body",
        "struct_body",
        "enum_body",
        "protocol_body",
        "extension_body",
    ];
    if recurse_kinds.contains(&node.kind()) || node.kind() == "source_file" {
        let cursor = &mut node.walk();
        for child in node.children(cursor) {
            extract_swift_node(child, source, file_str, file_path, parent_name, symbols);
        }
    }
}

fn extract_swift_func(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    parent_name: Option<&str>,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;

    let (qualified, kind) = match parent_name {
        Some(parent) => (format!("{}::{}", parent, name), SymbolKind::Method),
        None => (name.clone(), SymbolKind::Function),
    };

    let vis = detect_swift_visibility(node, source);
    let sig = extract_swift_signature(node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &qualified, &kind),
        name,
        qualified_name: qualified,
        kind,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(sig),
        doc_comment: extract_swift_doc(node, source),
        visibility: vis,
        parent: parent_name.map(|p| SymbolId::new(file_str, p, &SymbolKind::Class)),
        children: vec![],
    })
}

fn extract_swift_type(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    kind: SymbolKind,
    symbols: &mut Vec<Symbol>,
) {
    let name_node = match node.child_by_field_name("name") {
        Some(n) => n,
        None => return,
    };
    let name = match node_text(name_node, source) {
        Some(n) => n,
        None => return,
    };

    let vis = detect_swift_visibility(node, source);
    symbols.push(Symbol {
        id: SymbolId::new(file_str, &name, &kind),
        name: name.clone(),
        qualified_name: name.clone(),
        kind,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: None,
        doc_comment: extract_swift_doc(node, source),
        visibility: vis,
        parent: None,
        children: vec![],
    });

    // Recurse into body for methods
    if let Some(body) = node.child_by_field_name("body") {
        let cursor = &mut body.walk();
        for child in body.children(cursor) {
            extract_swift_node(child, source, file_str, file_path, Some(&name), symbols);
        }
    }
}

fn detect_swift_visibility(node: tree_sitter::Node, source: &[u8]) -> Visibility {
    // Check modifiers
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        if (child.kind() == "modifiers" || child.kind() == "modifier")
            && let Some(text) = node_text(child, source)
        {
            if text.contains("public") || text.contains("open") {
                return Visibility::Public;
            }
            if text.contains("internal") {
                return Visibility::Internal;
            }
            if text.contains("private") || text.contains("fileprivate") {
                return Visibility::Private;
            }
        }
    }
    Visibility::Internal // Swift default is internal
}

fn extract_swift_signature(node: tree_sitter::Node, source: &[u8]) -> String {
    let start = node.start_byte();
    let mut end = node.end_byte();
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        if child.kind() == "function_body" || child.kind() == "code_block" {
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

fn extract_swift_doc(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut sibling = node.prev_sibling();
    let mut comments = Vec::new();
    while let Some(s) = sibling {
        if s.kind() == "comment" {
            let text = node_text(s, source)?;
            if text.starts_with("///") {
                comments.push(text.trim_start_matches("///").trim().to_string());
            } else {
                break;
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

fn collect_swift_ids(
    node: tree_sitter::Node,
    source: &[u8],
    target: &str,
    lines: &[&str],
    refs: &mut Vec<IdentifierRef>,
) {
    match node.kind() {
        "comment" | "line_string_literal" | "multi_line_string_literal" => return,
        _ => {}
    }

    if node.kind() == "simple_identifier" && node_text(node, source).as_deref() == Some(target) {
        let line = node.start_position().row as u32 + 1;
        let context = lines
            .get(line as usize - 1)
            .unwrap_or(&"")
            .trim()
            .to_string();
        let kind = if let Some(p) = node.parent() {
            match p.kind() {
                "call_expression" => RefKind::Call,
                "import_declaration" => RefKind::Import,
                "type_identifier" | "type_annotation" => RefKind::TypeRef,
                "function_declaration"
                | "class_declaration"
                | "struct_declaration"
                | "enum_declaration"
                | "protocol_declaration" => {
                    if p.child_by_field_name("name").map(|n| n.id()) == Some(node.id()) {
                        RefKind::Definition
                    } else {
                        RefKind::Unknown
                    }
                }
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
        collect_swift_ids(child, source, target, lines, refs);
    }
}

// ─── Call extraction ────────────────────────────────────────────────

fn collect_swift_calls(
    node: tree_sitter::Node,
    source: &[u8],
    current_fn: Option<&str>,
    edges: &mut Vec<CallEdge>,
) {
    let mut fn_name: Option<String> = current_fn.map(|s| s.to_string());
    if node.kind() == "function_declaration"
        && let Some(name_node) = node.child_by_field_name("name")
        && let Some(name) = node_text(name_node, source)
    {
        fn_name = Some(name);
    }

    if node.kind() == "call_expression"
        && let Some(ref caller) = fn_name
        && let Some(func_node) = node.child(0)
        && let Some(callee) = node_text(func_node, source)
    {
        let clean = callee.rsplit('.').next().unwrap_or(&callee).to_string();
        edges.push((caller.clone(), clean));
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_swift_calls(child, source, fn_name.as_deref(), edges);
    }
}
