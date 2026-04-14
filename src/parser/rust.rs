use crate::model::symbol::*;
use crate::parser::traits::{CallEdge, IdentifierRef, ImportInfo, LanguageParser, RefKind};
use std::path::Path;

use super::helpers::{node_span, node_text, node_text_first_line, parse_source};

pub struct RustParser;

impl LanguageParser for RustParser {
    fn extensions(&self) -> &[&str] {
        &["rs"]
    }

    fn extract_symbols(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<Symbol>> {
        let tree = parse_source(tree_sitter_rust::LANGUAGE.into(), source, file_path)?;

        let mut symbols = Vec::new();
        let file_str = file_path.to_string_lossy();
        extract_from_node(
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
        let tree = parse_source(tree_sitter_rust::LANGUAGE.into(), source, file_path)?;

        let mut edges = Vec::new();
        collect_calls(tree.root_node(), source, None, &mut edges);
        Ok(edges)
    }

    fn find_identifiers(
        &self,
        source: &[u8],
        target_name: &str,
    ) -> anyhow::Result<Vec<IdentifierRef>> {
        let tree = parse_source(tree_sitter_rust::LANGUAGE.into(), source, Path::new(""))?;

        let mut refs = Vec::new();
        let lines: Vec<&str> = std::str::from_utf8(source).unwrap_or("").lines().collect();
        collect_identifiers(tree.root_node(), source, target_name, &lines, &mut refs);
        Ok(refs)
    }

    fn extract_imports(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<ImportInfo>> {
        let tree = parse_source(tree_sitter_rust::LANGUAGE.into(), source, file_path)?;

        let mut imports = Vec::new();
        collect_use_declarations(tree.root_node(), source, &mut imports);
        Ok(imports)
    }
}

fn extract_from_node(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    parent_name: Option<&str>,
    symbols: &mut Vec<Symbol>,
) {
    match node.kind() {
        "function_item" => {
            if let Some(sym) = extract_function(node, source, file_str, file_path, parent_name) {
                symbols.push(sym);
            }
        }
        "struct_item" => {
            if let Some(sym) = extract_struct(node, source, file_str, file_path) {
                let struct_name = sym.name.clone();
                symbols.push(sym);
                // Extract fields
                if let Some(body) = node.child_by_field_name("body") {
                    extract_fields(body, source, file_str, file_path, &struct_name, symbols);
                }
            }
        }
        "enum_item" => {
            if let Some(sym) = extract_enum(node, source, file_str, file_path) {
                let enum_name = sym.name.clone();
                symbols.push(sym);
                // Extract variants
                if let Some(body) = node.child_by_field_name("body") {
                    extract_enum_variants(body, source, file_str, file_path, &enum_name, symbols);
                }
            }
        }
        "trait_item" => {
            if let Some(sym) = extract_trait(node, source, file_str, file_path) {
                symbols.push(sym);
            }
        }
        "impl_item" => {
            extract_impl(node, source, file_str, file_path, symbols);
        }
        "const_item" | "static_item" => {
            if let Some(sym) = extract_const(node, source, file_str, file_path) {
                symbols.push(sym);
            }
        }
        "type_item" => {
            if let Some(sym) = extract_type_alias(node, source, file_str, file_path) {
                symbols.push(sym);
            }
        }
        "macro_definition" => {
            if let Some(sym) = extract_macro(node, source, file_str, file_path) {
                symbols.push(sym);
            }
        }
        _ => {}
    }

    // Recurse into children (but not into function/impl bodies for top-level scan)
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        match child.kind() {
            // Don't recurse into bodies — we handle those specifically
            "block" | "declaration_list" | "field_declaration_list" | "enum_variant_list" => {}
            _ => extract_from_node(child, source, file_str, file_path, parent_name, symbols),
        }
    }
}

fn extract_function(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    parent_name: Option<&str>,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    let vis = extract_visibility(node, source);
    let doc = extract_doc_comment(node, source);
    let sig = extract_signature(node, source);

    let (qualified, kind) = match parent_name {
        Some(parent) => (format!("{}::{}", parent, name), SymbolKind::Method),
        None => (name.clone(), SymbolKind::Function),
    };

    let parent_id = parent_name.map(|p| SymbolId::new(file_str, p, &SymbolKind::Struct));

    Some(Symbol {
        id: SymbolId::new(file_str, &qualified, &kind),
        name,
        qualified_name: qualified,
        kind,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(sig),
        doc_comment: doc,
        visibility: vis,
        parent: parent_id,
        children: vec![],
    })
}

fn extract_struct(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    let vis = extract_visibility(node, source);
    let doc = extract_doc_comment(node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Struct),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Struct,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(format!(
            "struct {}",
            node_text(name_node, source).unwrap_or_default()
        )),
        doc_comment: doc,
        visibility: vis,
        parent: None,
        children: vec![],
    })
}

fn extract_enum(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    let vis = extract_visibility(node, source);
    let doc = extract_doc_comment(node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Enum),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Enum,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: None,
        doc_comment: doc,
        visibility: vis,
        parent: None,
        children: vec![],
    })
}

fn extract_enum_variants(
    body: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    enum_name: &str,
    symbols: &mut Vec<Symbol>,
) {
    let cursor = &mut body.walk();
    for child in body.children(cursor) {
        if child.kind() == "enum_variant"
            && let Some(name_node) = child.child_by_field_name("name")
            && let Some(name) = node_text(name_node, source)
        {
            let qualified = format!("{}::{}", enum_name, name);
            symbols.push(Symbol {
                id: SymbolId::new(file_str, &qualified, &SymbolKind::EnumVariant),
                name,
                qualified_name: qualified,
                kind: SymbolKind::EnumVariant,
                file_path: file_path.to_path_buf(),
                span: node_span(child),
                signature: None,
                doc_comment: extract_doc_comment(child, source),
                visibility: Visibility::Public,
                parent: Some(SymbolId::new(file_str, enum_name, &SymbolKind::Enum)),
                children: vec![],
            });
        }
    }
}

fn extract_fields(
    body: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    struct_name: &str,
    symbols: &mut Vec<Symbol>,
) {
    let cursor = &mut body.walk();
    for child in body.children(cursor) {
        if child.kind() == "field_declaration"
            && let Some(name_node) = child.child_by_field_name("name")
            && let Some(name) = node_text(name_node, source)
        {
            let qualified = format!("{}::{}", struct_name, name);
            let vis = extract_visibility(child, source);
            let type_str = child
                .child_by_field_name("type")
                .and_then(|t| node_text(t, source));
            let sig = type_str.map(|t| format!("{}: {}", name, t));

            symbols.push(Symbol {
                id: SymbolId::new(file_str, &qualified, &SymbolKind::Field),
                name,
                qualified_name: qualified,
                kind: SymbolKind::Field,
                file_path: file_path.to_path_buf(),
                span: node_span(child),
                signature: sig,
                doc_comment: extract_doc_comment(child, source),
                visibility: vis,
                parent: Some(SymbolId::new(file_str, struct_name, &SymbolKind::Struct)),
                children: vec![],
            });
        }
    }
}

fn extract_trait(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    let vis = extract_visibility(node, source);
    let doc = extract_doc_comment(node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Interface),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Interface,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: None,
        doc_comment: doc,
        visibility: vis,
        parent: None,
        children: vec![],
    })
}

fn extract_impl(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    symbols: &mut Vec<Symbol>,
) {
    // Get the type name from impl
    let type_node = node.child_by_field_name("type");
    let type_name = type_node.and_then(|t| node_text(t, source));

    if let Some(body) = node.child_by_field_name("body") {
        let cursor = &mut body.walk();
        for child in body.children(cursor) {
            if child.kind() == "function_item"
                && let Some(sym) =
                    extract_function(child, source, file_str, file_path, type_name.as_deref())
            {
                symbols.push(sym);
            }
        }
    }
}

fn extract_const(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    let vis = extract_visibility(node, source);
    let sig = Some(node_text_first_line(node, source));

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Constant),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Constant,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: sig,
        doc_comment: extract_doc_comment(node, source),
        visibility: vis,
        parent: None,
        children: vec![],
    })
}

fn extract_type_alias(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    let vis = extract_visibility(node, source);
    let sig = Some(node_text_first_line(node, source));

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::TypeAlias),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::TypeAlias,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: sig,
        doc_comment: extract_doc_comment(node, source),
        visibility: vis,
        parent: None,
        children: vec![],
    })
}

fn extract_macro(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Macro),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Macro,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: None,
        doc_comment: extract_doc_comment(node, source),
        visibility: Visibility::Public,
        parent: None,
        children: vec![],
    })
}

// ─── Utility helpers ────────────────────────────────────────────────

fn extract_visibility(node: tree_sitter::Node, source: &[u8]) -> Visibility {
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        if child.kind() == "visibility_modifier" {
            let text = node_text(child, source).unwrap_or_default();
            if text.contains("crate") {
                return Visibility::Internal;
            }
            return Visibility::Public;
        }
    }
    Visibility::Private
}

fn extract_doc_comment(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    // Look at preceding siblings for doc comments
    let mut comments = Vec::new();
    let mut sibling = node.prev_sibling();
    while let Some(s) = sibling {
        if s.kind() == "line_comment" {
            let text = node_text(s, source)?;
            if text.starts_with("///") || text.starts_with("//!") {
                let cleaned = text
                    .trim_start_matches("///")
                    .trim_start_matches("//!")
                    .trim();
                comments.push(cleaned.to_string());
            } else {
                break;
            }
        } else if s.kind() == "attribute_item" {
            // Skip attributes (#[...])
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

fn extract_signature(node: tree_sitter::Node, source: &[u8]) -> String {
    // Get text from start of function to opening brace
    let start = node.start_byte();
    let mut end = node.end_byte();

    // Find the opening brace to truncate there
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        if child.kind() == "block" {
            end = child.start_byte();
            break;
        }
    }

    let sig_bytes = &source[start..end];
    let sig = String::from_utf8_lossy(sig_bytes).trim().to_string();

    // Clean up: remove trailing whitespace, normalize
    sig.lines().map(|l| l.trim()).collect::<Vec<_>>().join(" ")
}

// ─── Call graph extraction ──────────────────────────────────────────

/// Recursively collect call edges: (caller_function, callee_name).
fn collect_calls(
    node: tree_sitter::Node,
    source: &[u8],
    current_fn: Option<&str>,
    edges: &mut Vec<CallEdge>,
) {
    let mut fn_name: Option<String> = current_fn.map(|s| s.to_string());

    // Track which function we're inside
    if node.kind() == "function_item"
        && let Some(name_node) = node.child_by_field_name("name")
        && let Some(name) = node_text(name_node, source)
    {
        fn_name = Some(name);
    }

    // Detect call expressions
    if node.kind() == "call_expression"
        && let Some(ref caller) = fn_name
        && let Some(func_node) = node.child_by_field_name("function")
        && let Some(callee) = extract_callee_name(func_node, source)
    {
        edges.push((caller.clone(), callee));
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_calls(child, source, fn_name.as_deref(), edges);
    }
}

fn extract_callee_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => node_text(node, source),
        "field_expression" => {
            // e.g. self.process_block(...) → "process_block"
            node.child_by_field_name("field")
                .and_then(|f| node_text(f, source))
        }
        "scoped_identifier" => {
            // e.g. AudioEngine::new(...) → "AudioEngine::new"
            node_text(node, source)
        }
        _ => node_text(node, source),
    }
}

// ─── Identifier reference finding ───────────────────────────────────

/// Find all identifier nodes matching target_name, classifying each by AST context.
fn collect_identifiers(
    node: tree_sitter::Node,
    source: &[u8],
    target_name: &str,
    lines: &[&str],
    refs: &mut Vec<IdentifierRef>,
) {
    // Skip comments and string literals
    match node.kind() {
        "line_comment" | "block_comment" | "string_literal" | "raw_string_literal"
        | "char_literal" => {
            return;
        }
        _ => {}
    }

    if (node.kind() == "identifier" || node.kind() == "type_identifier")
        && node_text(node, source).as_deref() == Some(target_name)
    {
        let line = node.start_position().row as u32 + 1;
        let context = lines
            .get(line as usize - 1)
            .unwrap_or(&"")
            .trim()
            .to_string();

        let kind = classify_ref_context(node);

        refs.push(IdentifierRef {
            line,
            context,
            kind,
        });
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_identifiers(child, source, target_name, lines, refs);
    }
}

/// Classify a reference based on its AST parent/grandparent context.
fn classify_ref_context(node: tree_sitter::Node) -> RefKind {
    let parent = match node.parent() {
        Some(p) => p,
        None => return RefKind::Unknown,
    };

    match parent.kind() {
        // Function/method call: foo(...) or obj.foo(...)
        "call_expression" => RefKind::Call,

        // Field expression: self.foo or obj.foo
        // If grandparent is call_expression, it's a method call
        "field_expression" => {
            if let Some(gp) = parent.parent()
                && gp.kind() == "call_expression"
            {
                return RefKind::Call;
            }
            RefKind::FieldAccess
        }

        // Scoped identifier: Foo::bar, crate::Foo
        "scoped_identifier" => {
            if let Some(gp) = parent.parent() {
                match gp.kind() {
                    "call_expression" => return RefKind::Call,
                    "use_declaration" | "use_list" | "scoped_use_list" => return RefKind::Import,
                    _ => {}
                }
            }
            // If node is type_identifier, likely a type ref
            if node.kind() == "type_identifier" {
                RefKind::TypeRef
            } else {
                RefKind::Unknown
            }
        }

        // Use declaration
        "use_declaration" | "use_list" | "scoped_use_list" | "use_as_clause" => RefKind::Import,

        // Type positions
        "type_identifier" | "generic_type" | "reference_type" | "type_arguments"
        | "type_parameters" | "bounded_type" | "function_type" => RefKind::TypeRef,

        // Parameter types, return types, field types
        "parameter" | "return_type" | "field_declaration" => {
            if node.kind() == "type_identifier" {
                RefKind::TypeRef
            } else {
                RefKind::Unknown
            }
        }

        // Struct expression: Foo { ... } or Foo::new(...)
        "struct_expression" => RefKind::Constructor,

        // Definition sites
        "function_item" | "struct_item" | "enum_item" | "trait_item" | "type_item"
        | "const_item" | "static_item" | "macro_definition" => {
            // Check if this node is the "name" field of the definition
            if let Some(name_node) = parent.child_by_field_name("name")
                && name_node.id() == node.id()
            {
                return RefKind::Definition;
            }
            RefKind::Unknown
        }

        // impl block type
        "impl_item" => {
            if let Some(type_node) = parent.child_by_field_name("type")
                && type_node.id() == node.id()
            {
                return RefKind::TypeRef;
            }
            RefKind::Unknown
        }

        _ => RefKind::Unknown,
    }
}

// ─── Import extraction ──────────────────────────────────────────────

/// Extract `use` declarations from Rust source.
fn collect_use_declarations(node: tree_sitter::Node, source: &[u8], imports: &mut Vec<ImportInfo>) {
    if node.kind() == "use_declaration"
        && let Some(text) = node_text(node, source)
    {
        let cleaned = text
            .trim_start_matches("use ")
            .trim_start_matches("pub use ")
            .trim_end_matches(';')
            .trim();

        // Handle "use crate::foo::bar::{A, B, C};"
        if let Some(brace_pos) = cleaned.find('{') {
            let module = cleaned[..brace_pos].trim_end_matches("::").trim();
            let names_str = &cleaned[brace_pos + 1..];
            let names_str = names_str.trim_end_matches('}');
            let names: Vec<String> = names_str
                .split(',')
                .map(|n| {
                    n.trim()
                        .split(" as ")
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string()
                })
                .filter(|n| !n.is_empty())
                .collect();
            imports.push(ImportInfo {
                module_path: module.to_string(),
                names,
            });
        } else {
            // Simple: "use crate::foo::Bar;" → module = "crate::foo", name = "Bar"
            let parts: Vec<&str> = cleaned.rsplitn(2, "::").collect();
            if parts.len() == 2 {
                let name = parts[0].split(" as ").next().unwrap_or(parts[0]).trim();
                let module = parts[1].trim();
                imports.push(ImportInfo {
                    module_path: module.to_string(),
                    names: vec![name.to_string()],
                });
            } else {
                imports.push(ImportInfo {
                    module_path: cleaned.to_string(),
                    names: vec![],
                });
            }
        }
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_use_declarations(child, source, imports);
    }
}
