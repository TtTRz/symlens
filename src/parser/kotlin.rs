use crate::model::symbol::*;
use crate::parser::traits::{CallEdge, IdentifierRef, ImportInfo, LanguageParser, RefKind};
use std::path::Path;

pub struct KotlinParser;

impl LanguageParser for KotlinParser {
    fn extensions(&self) -> &[&str] {
        &["kt", "kts"]
    }

    fn extract_symbols(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<Symbol>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_kotlin_ng::LANGUAGE.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse {}", file_path.display()))?;

        let mut symbols = Vec::new();
        let file_str = file_path.to_string_lossy();
        extract_kotlin_node(
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
        parser.set_language(&tree_sitter_kotlin_ng::LANGUAGE.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse {}", file_path.display()))?;

        let mut edges = Vec::new();
        collect_kotlin_calls(tree.root_node(), source, None, &mut edges);
        Ok(edges)
    }

    fn find_identifiers(
        &self,
        source: &[u8],
        target_name: &str,
    ) -> anyhow::Result<Vec<IdentifierRef>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_kotlin_ng::LANGUAGE.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("parse failed"))?;

        let mut refs = Vec::new();
        let lines: Vec<&str> = std::str::from_utf8(source).unwrap_or("").lines().collect();
        collect_kotlin_ids(tree.root_node(), source, target_name, &lines, &mut refs);
        Ok(refs)
    }

    fn extract_imports(&self, source: &[u8], _file_path: &Path) -> anyhow::Result<Vec<ImportInfo>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_kotlin_ng::LANGUAGE.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("parse failed"))?;

        let mut imports = Vec::new();
        collect_kotlin_imports(tree.root_node(), source, &mut imports);
        Ok(imports)
    }
}

// ─── Symbol extraction ──────────────────────────────────────────────

fn extract_kotlin_node(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    parent_name: Option<&str>,
    symbols: &mut Vec<Symbol>,
) {
    match node.kind() {
        "class_declaration" => {
            // tree-sitter-kotlin-ng: both classes and interfaces are class_declaration.
            // Interfaces have an `interface` keyword child instead of `class`.
            let is_interface = has_keyword_child(node, "interface");
            let is_enum = has_enum_modifier(node, source);

            let kind = if is_enum {
                SymbolKind::Enum
            } else if is_interface {
                SymbolKind::Interface
            } else {
                SymbolKind::Class
            };

            if let Some(name) = find_child_text_by_kind(node, "identifier", source) {
                let doc = extract_kotlin_doc(node, source);
                let sig =
                    extract_kotlin_signature(node, source, &["class_body", "enum_class_body"]);
                let vis = extract_visibility(node, source);

                symbols.push(Symbol {
                    id: SymbolId::new(file_str, &name, &kind),
                    name: name.clone(),
                    qualified_name: name.clone(),
                    kind,
                    file_path: file_path.to_path_buf(),
                    span: node_span(node),
                    signature: Some(sig),
                    doc_comment: doc,
                    visibility: vis,
                    parent: None,
                    children: vec![],
                });

                if let Some(body) = find_child_by_kind(node, "class_body")
                    .or_else(|| find_child_by_kind(node, "enum_class_body"))
                {
                    let cursor = &mut body.walk();
                    for child in body.children(cursor) {
                        extract_kotlin_node(
                            child,
                            source,
                            file_str,
                            file_path,
                            Some(&name),
                            symbols,
                        );
                    }
                }
                return;
            }
        }
        "object_declaration" => {
            if let Some(name) = find_child_text_by_kind(node, "identifier", source) {
                let doc = extract_kotlin_doc(node, source);
                let sig = extract_kotlin_signature(node, source, &["class_body"]);
                let vis = extract_visibility(node, source);

                symbols.push(Symbol {
                    id: SymbolId::new(file_str, &name, &SymbolKind::Class),
                    name: name.clone(),
                    qualified_name: name.clone(),
                    kind: SymbolKind::Class,
                    file_path: file_path.to_path_buf(),
                    span: node_span(node),
                    signature: Some(sig),
                    doc_comment: doc,
                    visibility: vis,
                    parent: None,
                    children: vec![],
                });

                if let Some(body) = find_child_by_kind(node, "class_body") {
                    let cursor = &mut body.walk();
                    for child in body.children(cursor) {
                        extract_kotlin_node(
                            child,
                            source,
                            file_str,
                            file_path,
                            Some(&name),
                            symbols,
                        );
                    }
                }
                return;
            }
        }
        "function_declaration" => {
            if let Some(name) = find_child_text_by_kind(node, "identifier", source) {
                let doc = extract_kotlin_doc(node, source);
                let sig = extract_kotlin_signature(node, source, &["function_body"]);
                let vis = extract_visibility(node, source);

                let (kind, qualified) = match parent_name {
                    Some(cls) => (SymbolKind::Method, format!("{}::{}", cls, name)),
                    None => (SymbolKind::Function, name.clone()),
                };

                symbols.push(Symbol {
                    id: SymbolId::new(file_str, &qualified, &kind),
                    name,
                    qualified_name: qualified,
                    kind,
                    file_path: file_path.to_path_buf(),
                    span: node_span(node),
                    signature: Some(sig),
                    doc_comment: doc,
                    visibility: vis,
                    parent: parent_name.map(|c| SymbolId::new(file_str, c, &SymbolKind::Class)),
                    children: vec![],
                });
                return;
            }
        }
        "property_declaration" => {
            let name = find_child_by_kind(node, "variable_declaration")
                .and_then(|vd| find_child_text_by_kind(vd, "identifier", source))
                .or_else(|| find_child_text_by_kind(node, "identifier", source));

            if let Some(name) = name {
                let doc = extract_kotlin_doc(node, source);
                let vis = extract_visibility(node, source);

                let has_val = has_keyword_child(node, "val");
                let has_const = has_const_modifier(node, source);

                let kind = if has_const || (has_val && parent_name.is_none()) {
                    SymbolKind::Constant
                } else {
                    SymbolKind::Variable
                };

                let qualified = match parent_name {
                    Some(cls) => format!("{}::{}", cls, name),
                    None => name.clone(),
                };

                symbols.push(Symbol {
                    id: SymbolId::new(file_str, &qualified, &kind),
                    name,
                    qualified_name: qualified,
                    kind,
                    file_path: file_path.to_path_buf(),
                    span: node_span(node),
                    signature: Some(node_text_first_line(node, source)),
                    doc_comment: doc,
                    visibility: vis,
                    parent: parent_name.map(|c| SymbolId::new(file_str, c, &SymbolKind::Class)),
                    children: vec![],
                });
                return;
            }
        }
        _ => {}
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        extract_kotlin_node(child, source, file_str, file_path, parent_name, symbols);
    }
}

fn has_keyword_child(node: tree_sitter::Node, keyword: &str) -> bool {
    let cursor = &mut node.walk();
    node.children(cursor).any(|child| child.kind() == keyword)
}

fn has_enum_modifier(node: tree_sitter::Node, source: &[u8]) -> bool {
    if let Some(mods) = find_child_by_kind(node, "modifiers") {
        let cursor = &mut mods.walk();
        for child in mods.children(cursor) {
            if child.kind() == "class_modifier"
                && let Some(text) = node_text(child, source)
                && text.trim() == "enum"
            {
                return true;
            }
        }
    }
    false
}

fn has_const_modifier(node: tree_sitter::Node, source: &[u8]) -> bool {
    if let Some(mods) = find_child_by_kind(node, "modifiers") {
        let cursor = &mut mods.walk();
        for child in mods.children(cursor) {
            if child.kind() == "property_modifier"
                && let Some(text) = node_text(child, source)
                && text.trim() == "const"
            {
                return true;
            }
        }
    }
    false
}

fn extract_visibility(node: tree_sitter::Node, source: &[u8]) -> Visibility {
    if let Some(mods) = find_child_by_kind(node, "modifiers") {
        let cursor = &mut mods.walk();
        for child in mods.children(cursor) {
            if child.kind() == "visibility_modifier"
                && let Some(text) = node_text(child, source)
            {
                return match text.trim() {
                    "private" => Visibility::Private,
                    "internal" => Visibility::Internal,
                    "protected" => Visibility::Internal,
                    "public" => Visibility::Public,
                    _ => Visibility::Public,
                };
            }
        }
    }
    Visibility::Public
}

// ─── Call extraction ────────────────────────────────────────────────

fn collect_kotlin_calls(
    node: tree_sitter::Node,
    source: &[u8],
    current_fn: Option<&str>,
    edges: &mut Vec<CallEdge>,
) {
    let mut fn_name = current_fn;

    if node.kind() == "function_declaration"
        && let Some(name) = find_child_text_by_kind(node, "identifier", source)
    {
        fn_name = Some(Box::leak(name.into_boxed_str()));
    }

    if node.kind() == "call_expression"
        && let Some(caller) = fn_name
        && let Some(callee) = extract_call_name(node, source)
    {
        edges.push((caller.to_string(), callee));
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_kotlin_calls(child, source, fn_name, edges);
    }
}

fn extract_call_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let first = node.child(0)?;
    match first.kind() {
        "identifier" => node_text(first, source),
        "navigation_expression" => {
            let cursor = &mut first.walk();
            let mut last_id = None;
            for child in first.children(cursor) {
                if child.kind() == "identifier" {
                    last_id = node_text(child, source);
                }
            }
            last_id
        }
        _ => node_text(first, source),
    }
}

// ─── Identifier references ──────────────────────────────────────────

fn collect_kotlin_ids(
    node: tree_sitter::Node,
    source: &[u8],
    target: &str,
    lines: &[&str],
    refs: &mut Vec<IdentifierRef>,
) {
    match node.kind() {
        "line_comment" | "block_comment" | "string_literal" => return,
        _ => {}
    }

    if node.kind() == "identifier" && node_text(node, source).as_deref() == Some(target) {
        let line = node.start_position().row as u32 + 1;
        let context = lines
            .get(line as usize - 1)
            .unwrap_or(&"")
            .trim()
            .to_string();
        let kind = classify_kotlin_ref(node);
        refs.push(IdentifierRef {
            line,
            context,
            kind,
        });
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_kotlin_ids(child, source, target, lines, refs);
    }
}

fn classify_kotlin_ref(node: tree_sitter::Node) -> RefKind {
    let parent = match node.parent() {
        Some(p) => p,
        None => return RefKind::Unknown,
    };

    match parent.kind() {
        "import" => RefKind::Import,
        "qualified_identifier" => {
            if let Some(gp) = parent.parent()
                && gp.kind() == "import"
            {
                return RefKind::Import;
            }
            RefKind::Unknown
        }
        "function_declaration" => {
            if find_child_by_kind(parent, "identifier").map(|n| n.id()) == Some(node.id()) {
                RefKind::Definition
            } else {
                RefKind::TypeRef
            }
        }
        "class_declaration" | "object_declaration" => {
            if find_child_by_kind(parent, "identifier").map(|n| n.id()) == Some(node.id()) {
                RefKind::Definition
            } else {
                RefKind::TypeRef
            }
        }
        "user_type" | "type_reference" | "type_projection" | "nullable_type" => RefKind::TypeRef,
        "call_expression" => {
            if node.id() == parent.child(0).map(|c| c.id()).unwrap_or(0) {
                RefKind::Call
            } else {
                RefKind::Unknown
            }
        }
        "navigation_expression" => {
            if let Some(gp) = parent.parent()
                && gp.kind() == "call_expression"
            {
                return RefKind::Call;
            }
            RefKind::FieldAccess
        }
        "variable_declaration" => {
            if find_child_by_kind(parent, "identifier").map(|n| n.id()) == Some(node.id()) {
                RefKind::Definition
            } else {
                RefKind::TypeRef
            }
        }
        "property_declaration" => {
            if find_child_by_kind(parent, "variable_declaration")
                .and_then(|vd| find_child_by_kind(vd, "identifier"))
                .map(|n| n.id())
                == Some(node.id())
            {
                RefKind::Definition
            } else {
                RefKind::TypeRef
            }
        }
        _ => RefKind::Unknown,
    }
}

// ─── Import extraction ──────────────────────────────────────────────

fn collect_kotlin_imports(node: tree_sitter::Node, source: &[u8], imports: &mut Vec<ImportInfo>) {
    if node.kind() == "import"
        && let Some(qi) = find_child_by_kind(node, "qualified_identifier")
        && let Some(path_text) = node_text(qi, source)
    {
        let path: String = path_text.chars().filter(|c| !c.is_whitespace()).collect();
        let name = path.rsplit('.').next().unwrap_or(&path).to_string();
        imports.push(ImportInfo {
            module_path: path,
            names: vec![name],
        });
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_kotlin_imports(child, source, imports);
    }
}

// ─── Helpers ────────────────────────────────────────────────────────

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

fn find_child_by_kind<'a>(
    node: tree_sitter::Node<'a>,
    kind: &str,
) -> Option<tree_sitter::Node<'a>> {
    let cursor = &mut node.walk();
    node.children(cursor).find(|&child| child.kind() == kind)
}

fn find_child_text_by_kind(node: tree_sitter::Node, kind: &str, source: &[u8]) -> Option<String> {
    find_child_by_kind(node, kind).and_then(|n| node_text(n, source))
}

fn node_text_first_line(node: tree_sitter::Node, source: &[u8]) -> String {
    node.utf8_text(source)
        .ok()
        .and_then(|t| t.lines().next().map(|l| l.trim().to_string()))
        .unwrap_or_default()
}

fn extract_kotlin_signature(node: tree_sitter::Node, source: &[u8], body_kinds: &[&str]) -> String {
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

fn extract_kotlin_doc(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut comments = Vec::new();
    let mut sibling = node.prev_sibling();
    while let Some(s) = sibling {
        match s.kind() {
            "block_comment" => {
                if let Some(text) = node_text(s, source) {
                    if text.starts_with("/**") {
                        let cleaned = text
                            .trim_start_matches("/**")
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
            }
            "line_comment" => {
                if let Some(text) = node_text(s, source) {
                    let cleaned = text
                        .trim_start_matches("///")
                        .trim_start_matches("//")
                        .trim();
                    comments.push(cleaned.to_string());
                }
            }
            _ => break,
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
