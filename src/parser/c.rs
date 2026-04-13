use crate::model::symbol::*;
use crate::parser::traits::{CallEdge, IdentifierRef, ImportInfo, LanguageParser, RefKind};
use std::path::Path;

pub struct CParser;

impl LanguageParser for CParser {
    fn extensions(&self) -> &[&str] {
        &["c", "h"]
    }

    fn extract_symbols(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<Symbol>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_c::LANGUAGE.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse {}", file_path.display()))?;

        let mut symbols = Vec::new();
        let file_str = file_path.to_string_lossy();
        extract_c_node(tree.root_node(), source, &file_str, file_path, &mut symbols);
        Ok(symbols)
    }

    fn extract_calls(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<CallEdge>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_c::LANGUAGE.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse {}", file_path.display()))?;

        let mut edges = Vec::new();
        collect_c_calls(tree.root_node(), source, None, &mut edges);
        Ok(edges)
    }

    fn find_identifiers(
        &self,
        source: &[u8],
        target_name: &str,
    ) -> anyhow::Result<Vec<IdentifierRef>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_c::LANGUAGE.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("parse failed"))?;

        let mut refs = Vec::new();
        let lines: Vec<&str> = std::str::from_utf8(source).unwrap_or("").lines().collect();
        collect_c_ids(tree.root_node(), source, target_name, &lines, &mut refs);
        Ok(refs)
    }

    fn extract_imports(&self, source: &[u8], _file_path: &Path) -> anyhow::Result<Vec<ImportInfo>> {
        // C #include directives are preprocessor directives; use text-based scan.
        let text = std::str::from_utf8(source).unwrap_or("");
        let mut imports = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("#include") {
                let path = trimmed
                    .trim_start_matches("#include")
                    .trim()
                    .trim_matches(|c| c == '<' || c == '>' || c == '"');
                let name = path.rsplit('/').next().unwrap_or(path).to_string();
                imports.push(ImportInfo {
                    module_path: path.to_string(),
                    names: vec![name],
                });
            }
        }
        Ok(imports)
    }
}

fn extract_c_node(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    symbols: &mut Vec<Symbol>,
) {
    match node.kind() {
        "function_definition" => {
            if let Some(sym) = extract_c_func(node, source, file_str, file_path) {
                symbols.push(sym);
            }
        }
        "struct_specifier" => {
            if let Some(sym) =
                extract_c_struct_or_enum(node, source, file_str, file_path, SymbolKind::Struct)
            {
                symbols.push(sym);
            }
        }
        "enum_specifier" => {
            if let Some(sym) =
                extract_c_struct_or_enum(node, source, file_str, file_path, SymbolKind::Enum)
            {
                symbols.push(sym);
            }
        }
        "type_definition" => {
            if let Some(sym) = extract_c_typedef(node, source, file_str, file_path) {
                symbols.push(sym);
            }
        }
        "preproc_def" | "preproc_function_def" => {
            if let Some(sym) = extract_c_macro(node, source, file_str, file_path) {
                symbols.push(sym);
            }
        }
        "declaration" => {
            // Top-level variable declarations with initializers
            if node.parent().map(|p| p.kind()) == Some("translation_unit")
                && let Some(sym) = extract_c_variable(node, source, file_str, file_path)
            {
                symbols.push(sym);
            }
        }
        _ => {}
    }

    // Recurse into translation_unit (root)
    if node.kind() == "translation_unit" {
        let cursor = &mut node.walk();
        for child in node.children(cursor) {
            extract_c_node(child, source, file_str, file_path, symbols);
        }
    }
}

fn extract_c_func(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name = extract_function_name(node, source)?;
    let sig = extract_c_signature(node, source);
    let doc = extract_c_doc(node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Function),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Function,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(sig),
        doc_comment: doc,
        visibility: Visibility::Public,
        parent: None,
        children: vec![],
    })
}

/// Navigate the declarator chain to find the function name.
/// function_definition → declarator (function_declarator) → declarator (identifier)
/// May also encounter pointer_declarator wrapping function_declarator.
fn extract_function_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let declarator = node.child_by_field_name("declarator")?;
    find_identifier_in_declarator(declarator, source)
}

fn find_identifier_in_declarator(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => node_text(node, source),
        "function_declarator" => {
            let inner = node.child_by_field_name("declarator")?;
            find_identifier_in_declarator(inner, source)
        }
        "pointer_declarator" => {
            let inner = node.child_by_field_name("declarator")?;
            find_identifier_in_declarator(inner, source)
        }
        "parenthesized_declarator" => {
            let cursor = &mut node.walk();
            for child in node.children(cursor) {
                if let Some(name) = find_identifier_in_declarator(child, source) {
                    return Some(name);
                }
            }
            None
        }
        _ => {
            // Try declarator field first, then iterate children
            if let Some(inner) = node.child_by_field_name("declarator") {
                return find_identifier_in_declarator(inner, source);
            }
            let cursor = &mut node.walk();
            for child in node.children(cursor) {
                if let Some(name) = find_identifier_in_declarator(child, source) {
                    return Some(name);
                }
            }
            None
        }
    }
}

fn extract_c_struct_or_enum(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    kind: SymbolKind,
) -> Option<Symbol> {
    // The name is the first type_identifier child
    let name = find_child_by_kind(node, "type_identifier").and_then(|n| node_text(n, source))?;
    let doc = extract_c_doc(node, source);
    let keyword = if kind == SymbolKind::Struct {
        "struct"
    } else {
        "enum"
    };

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &kind),
        name: name.clone(),
        qualified_name: name.clone(),
        kind,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(format!("{} {}", keyword, name)),
        doc_comment: doc,
        visibility: Visibility::Public,
        parent: None,
        children: vec![],
    })
}

fn extract_c_typedef(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    // Check if the typedef wraps a struct or enum specifier
    let mut inner_kind: Option<SymbolKind> = None;
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        match child.kind() {
            "struct_specifier" => {
                inner_kind = Some(SymbolKind::Struct);
            }
            "enum_specifier" => {
                inner_kind = Some(SymbolKind::Enum);
            }
            _ => {}
        }
    }

    // For the typedef name, get the last type_identifier in the node
    let name = last_child_by_kind(node, "type_identifier").and_then(|n| node_text(n, source))?;

    let kind = inner_kind.unwrap_or(SymbolKind::TypeAlias);
    let doc = extract_c_doc(node, source);

    let sig = node
        .utf8_text(source)
        .ok()
        .and_then(|t| t.lines().next().map(|l| l.trim().to_string()));

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &kind),
        name: name.clone(),
        qualified_name: name,
        kind,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: sig,
        doc_comment: doc,
        visibility: Visibility::Public,
        parent: None,
        children: vec![],
    })
}

fn extract_c_macro(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    let doc = extract_c_doc(node, source);

    let sig = node.utf8_text(source).ok().map(|t| t.trim().to_string());

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Macro),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Macro,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: sig,
        doc_comment: doc,
        visibility: Visibility::Public,
        parent: None,
        children: vec![],
    })
}

fn extract_c_variable(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    // Look for an init_declarator child (declaration with initializer)
    let has_init = find_child_by_kind(node, "init_declarator").is_some();
    if !has_init {
        return None;
    }

    // Get the variable name from the declarator
    let init_decl = find_child_by_kind(node, "init_declarator")?;
    let name = find_identifier_in_declarator(
        init_decl
            .child_by_field_name("declarator")
            .unwrap_or(init_decl),
        source,
    )?;

    let sig = node
        .utf8_text(source)
        .ok()
        .and_then(|t| t.lines().next().map(|l| l.trim().to_string()));

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Variable),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Variable,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: sig,
        doc_comment: extract_c_doc(node, source),
        visibility: Visibility::Public,
        parent: None,
        children: vec![],
    })
}

fn extract_c_signature(node: tree_sitter::Node, source: &[u8]) -> String {
    let start = node.start_byte();
    let mut end = node.end_byte();
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        if child.kind() == "compound_statement" {
            end = child.start_byte();
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

fn extract_c_doc(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut comments = Vec::new();
    let mut sibling = node.prev_sibling();
    while let Some(s) = sibling {
        if s.kind() == "comment" {
            let text = node_text(s, source)?;
            let cleaned = text.trim_start_matches("//").trim();
            comments.push(cleaned.to_string());
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

// ─── Call extraction ────────────────────────────────────────────────

fn collect_c_calls(
    node: tree_sitter::Node,
    source: &[u8],
    current_fn: Option<&str>,
    edges: &mut Vec<CallEdge>,
) {
    let mut fn_name = current_fn;

    if node.kind() == "function_definition"
        && let Some(name) = extract_function_name(node, source)
    {
        fn_name = Some(Box::leak(name.into_boxed_str()));
    }

    if node.kind() == "call_expression"
        && let Some(caller) = fn_name
        && let Some(func_node) = node.child_by_field_name("function")
        && let Some(callee) = node_text(func_node, source)
    {
        edges.push((caller.to_string(), callee));
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_c_calls(child, source, fn_name, edges);
    }
}

// ─── Identifier references ──────────────────────────────────────────

fn collect_c_ids(
    node: tree_sitter::Node,
    source: &[u8],
    target: &str,
    lines: &[&str],
    refs: &mut Vec<IdentifierRef>,
) {
    match node.kind() {
        "comment" | "string_literal" | "char_literal" => return,
        _ => {}
    }

    if (node.kind() == "identifier"
        || node.kind() == "type_identifier"
        || node.kind() == "field_identifier")
        && node_text(node, source).as_deref() == Some(target)
    {
        let line = node.start_position().row as u32 + 1;
        let context = lines
            .get(line as usize - 1)
            .unwrap_or(&"")
            .trim()
            .to_string();
        let kind = classify_c_ref(node);
        refs.push(IdentifierRef {
            line,
            context,
            kind,
        });
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_c_ids(child, source, target, lines, refs);
    }
}

fn classify_c_ref(node: tree_sitter::Node) -> RefKind {
    let parent = match node.parent() {
        Some(p) => p,
        None => return RefKind::Unknown,
    };
    match parent.kind() {
        "call_expression" => RefKind::Call,
        "type_identifier" | "type_definition" | "parameter_declaration" | "field_declaration" => {
            RefKind::TypeRef
        }
        "function_declarator" => {
            // Check if this is the definition site
            if let Some(gp) = parent.parent()
                && gp.kind() == "function_definition"
            {
                return RefKind::Definition;
            }
            RefKind::Unknown
        }
        "function_definition" => RefKind::Definition,
        "preproc_include" => RefKind::Import,
        "field_expression" => {
            if let Some(gp) = parent.parent()
                && gp.kind() == "call_expression"
            {
                return RefKind::Call;
            }
            RefKind::FieldAccess
        }
        "init_declarator" | "struct_specifier" | "enum_specifier" => RefKind::Definition,
        _ => RefKind::Unknown,
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

fn last_child_by_kind<'a>(
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
