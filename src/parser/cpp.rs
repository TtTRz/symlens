use super::helpers::{node_span, node_text, parse_source};
use crate::model::symbol::*;
use crate::parser::traits::{CallEdge, IdentifierRef, ImportInfo, LanguageParser, RefKind};
use std::path::Path;

pub struct CppParser;

impl LanguageParser for CppParser {
    fn extensions(&self) -> &[&str] {
        &["cpp", "cc", "cxx", "hpp", "hh"]
    }

    fn extract_symbols(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<Symbol>> {
        let tree = parse_source(tree_sitter_cpp::LANGUAGE.into(), source, file_path)?;

        let mut symbols = Vec::new();
        let file_str = file_path.to_string_lossy();
        extract_cpp_node(
            tree.root_node(),
            source,
            &file_str,
            file_path,
            &mut symbols,
            None,
            Visibility::Public, // top-level default
        );
        Ok(symbols)
    }

    fn extract_calls(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<CallEdge>> {
        let tree = parse_source(tree_sitter_cpp::LANGUAGE.into(), source, file_path)?;

        let mut edges = Vec::new();
        collect_cpp_calls(tree.root_node(), source, None, &mut edges);
        Ok(edges)
    }

    fn find_identifiers(
        &self,
        source: &[u8],
        target_name: &str,
    ) -> anyhow::Result<Vec<IdentifierRef>> {
        let tree = parse_source(
            tree_sitter_cpp::LANGUAGE.into(),
            source,
            std::path::Path::new(""),
        )?;

        let mut refs = Vec::new();
        let lines: Vec<&str> = std::str::from_utf8(source).unwrap_or("").lines().collect();
        collect_cpp_ids(tree.root_node(), source, target_name, &lines, &mut refs);
        Ok(refs)
    }

    fn extract_imports(&self, source: &[u8], _file_path: &Path) -> anyhow::Result<Vec<ImportInfo>> {
        // #include directives are not AST nodes in tree-sitter-cpp in a reliable way,
        // so we do text-based scanning.
        let text = std::str::from_utf8(source).unwrap_or("");
        let mut imports = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("#include") {
                let rest = rest.trim();
                let path = if rest.starts_with('<') && rest.contains('>') {
                    rest.trim_start_matches('<')
                        .split('>')
                        .next()
                        .unwrap_or("")
                        .to_string()
                } else if rest.starts_with('"') {
                    rest.trim_matches('"').to_string()
                } else {
                    continue;
                };
                let name = path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&path)
                    .trim_end_matches(".h")
                    .trim_end_matches(".hpp")
                    .to_string();
                imports.push(ImportInfo {
                    module_path: path,
                    names: vec![name],
                });
            }
        }
        Ok(imports)
    }
}

// ─── Symbol extraction ─────────────────────────────────────────────

/// Context for tracking the current access specifier inside class/struct bodies.
struct ClassContext<'a> {
    name: &'a str,
    kind: SymbolKind, // Class or Struct
}

fn extract_cpp_node(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    symbols: &mut Vec<Symbol>,
    class_ctx: Option<&ClassContext>,
    default_vis: Visibility,
) {
    match node.kind() {
        "function_definition" => {
            if let Some(sym) =
                extract_cpp_function(node, source, file_str, file_path, class_ctx, default_vis)
            {
                symbols.push(sym);
            }
        }
        "class_specifier" => {
            extract_cpp_class_or_struct(
                node,
                source,
                file_str,
                file_path,
                symbols,
                SymbolKind::Class,
            );
        }
        "struct_specifier" => {
            extract_cpp_class_or_struct(
                node,
                source,
                file_str,
                file_path,
                symbols,
                SymbolKind::Struct,
            );
        }
        "enum_specifier" => {
            if let Some(sym) = extract_cpp_enum(node, source, file_str, file_path) {
                symbols.push(sym);
            }
        }
        "namespace_definition" => {
            if let Some(name_node) = node.child_by_field_name("name")
                && let Some(name) = node_text(name_node, source)
            {
                let doc = extract_cpp_doc(node, source);
                symbols.push(Symbol {
                    id: SymbolId::new(file_str, &name, &SymbolKind::Module),
                    name: name.clone(),
                    qualified_name: name,
                    kind: SymbolKind::Module,
                    file_path: file_path.to_path_buf(),
                    span: node_span(node),
                    signature: Some(format!(
                        "namespace {}",
                        node_text(name_node, source).unwrap_or_default()
                    )),
                    doc_comment: doc,
                    visibility: Visibility::Public,
                    parent: None,
                    children: vec![],
                });
            }
            // Recurse into namespace body
            if let Some(body) = node.child_by_field_name("body") {
                let cursor = &mut body.walk();
                for child in body.children(cursor) {
                    extract_cpp_node(
                        child,
                        source,
                        file_str,
                        file_path,
                        symbols,
                        None,
                        Visibility::Public,
                    );
                }
            }
            return; // don't recurse again below
        }
        "type_definition" | "alias_declaration" => {
            if let Some(sym) = extract_cpp_type_alias(node, source, file_str, file_path) {
                symbols.push(sym);
            }
        }
        "template_declaration" => {
            // Unwrap template and process the inner declaration
            let cursor = &mut node.walk();
            for child in node.children(cursor) {
                match child.kind() {
                    "class_specifier"
                    | "struct_specifier"
                    | "function_definition"
                    | "alias_declaration"
                    | "type_definition" => {
                        extract_cpp_node(
                            child,
                            source,
                            file_str,
                            file_path,
                            symbols,
                            class_ctx,
                            default_vis,
                        );
                    }
                    _ => {}
                }
            }
            return;
        }
        "declaration" if class_ctx.is_some() => {
            // field_declaration with function declarator → method declaration (pure virtual etc.)
            // Handled below in class body extraction
        }
        "field_declaration" if class_ctx.is_some() => {
            // Check if this is a method declaration (has function_declarator descendant)
            if has_function_declarator(node)
                && let Some(sym) = extract_cpp_field_method(
                    node,
                    source,
                    file_str,
                    file_path,
                    class_ctx,
                    default_vis,
                )
            {
                symbols.push(sym);
            }
        }
        _ => {}
    }

    // Recurse into top-level containers (translation_unit, declaration_list, etc.)
    if node.kind() == "translation_unit"
        || node.kind() == "declaration_list"
        || (class_ctx.is_none() && node.kind() == "compound_statement")
    {
        let cursor = &mut node.walk();
        for child in node.children(cursor) {
            extract_cpp_node(
                child,
                source,
                file_str,
                file_path,
                symbols,
                class_ctx,
                default_vis,
            );
        }
    }
}

fn extract_cpp_class_or_struct(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    symbols: &mut Vec<Symbol>,
    kind: SymbolKind,
) {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| node_text(n, source));
    let name = match name {
        Some(n) => n,
        None => return, // anonymous struct/class
    };

    let doc = extract_cpp_doc(node, source);
    let sig = format!(
        "{} {}",
        if kind == SymbolKind::Class {
            "class"
        } else {
            "struct"
        },
        &name
    );

    let mut children = Vec::new();

    // Extract members from body (field_declaration_list)
    if let Some(body) = node.child_by_field_name("body") {
        let default_vis = if kind == SymbolKind::Class {
            Visibility::Private
        } else {
            Visibility::Public
        };

        let ctx = ClassContext { name: &name, kind };

        let mut current_vis = default_vis;
        let cursor = &mut body.walk();
        for child in body.children(cursor) {
            if child.kind() == "access_specifier" {
                current_vis = parse_access_specifier(child, source);
            } else {
                let before = symbols.len();
                extract_cpp_node(
                    child,
                    source,
                    file_str,
                    file_path,
                    symbols,
                    Some(&ctx),
                    current_vis,
                );
                // Collect child IDs
                for sym in &symbols[before..] {
                    children.push(sym.id.clone());
                }
            }
        }
    }

    symbols.push(Symbol {
        id: SymbolId::new(file_str, &name, &kind),
        name: name.clone(),
        qualified_name: name,
        kind,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(sig),
        doc_comment: doc,
        visibility: Visibility::Public,
        parent: None,
        children,
    });
}

fn extract_cpp_function(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    class_ctx: Option<&ClassContext>,
    default_vis: Visibility,
) -> Option<Symbol> {
    let declarator = node.child_by_field_name("declarator")?;
    let name = extract_declarator_name(declarator, source)?;

    let (kind, qualified, parent) = match class_ctx {
        Some(ctx) => {
            let qn = format!("{}::{}", ctx.name, name);
            let parent_id = SymbolId::new(file_str, ctx.name, &ctx.kind);
            (SymbolKind::Method, qn, Some(parent_id))
        }
        None => (SymbolKind::Function, name.clone(), None),
    };

    let sig = extract_cpp_signature(node, source);
    let doc = extract_cpp_doc(node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &qualified, &kind),
        name,
        qualified_name: qualified,
        kind,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(sig),
        doc_comment: doc,
        visibility: default_vis,
        parent,
        children: vec![],
    })
}

fn extract_cpp_field_method(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    class_ctx: Option<&ClassContext>,
    default_vis: Visibility,
) -> Option<Symbol> {
    let name = find_function_name_in_field(node, source)?;

    let (qualified, parent) = match class_ctx {
        Some(ctx) => {
            let qn = format!("{}::{}", ctx.name, name);
            let parent_id = SymbolId::new(file_str, ctx.name, &ctx.kind);
            (qn, Some(parent_id))
        }
        None => (name.clone(), None),
    };

    let sig = node
        .utf8_text(source)
        .ok()
        .map(|t| t.trim().trim_end_matches(';').trim().to_string())
        .unwrap_or_default();
    let doc = extract_cpp_doc(node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &qualified, &SymbolKind::Method),
        name,
        qualified_name: qualified,
        kind: SymbolKind::Method,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(sig),
        doc_comment: doc,
        visibility: default_vis,
        parent,
        children: vec![],
    })
}

fn extract_cpp_enum(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    let doc = extract_cpp_doc(node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Enum),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Enum,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(format!("enum class {}", node_text(name_node, source)?)),
        doc_comment: doc,
        visibility: Visibility::Public,
        parent: None,
        children: vec![],
    })
}

fn extract_cpp_type_alias(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    // using Foo = Bar; → alias_declaration with name field
    // typedef int Foo; → type_definition with declarator field
    let name = if node.kind() == "alias_declaration" {
        node.child_by_field_name("name")
            .and_then(|n| node_text(n, source))
    } else {
        // type_definition: look for the declarator (type_identifier)
        node.child_by_field_name("declarator")
            .and_then(|n| node_text(n, source))
    };
    let name = name?;
    let doc = extract_cpp_doc(node, source);
    let sig = node
        .utf8_text(source)
        .ok()
        .map(|t| t.trim().trim_end_matches(';').trim().to_string());

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::TypeAlias),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::TypeAlias,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: sig,
        doc_comment: doc,
        visibility: Visibility::Public,
        parent: None,
        children: vec![],
    })
}

// ─── Helpers ────────────────────────────────────────────────────────

fn extract_declarator_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    // Walk down through function_declarator, qualified_identifier, etc.
    match node.kind() {
        "function_declarator" => {
            let decl = node.child_by_field_name("declarator")?;
            extract_declarator_name(decl, source)
        }
        "qualified_identifier" => {
            // Use the rightmost name component
            let name_node = node.child_by_field_name("name")?;
            node_text(name_node, source)
        }
        "identifier" | "field_identifier" | "type_identifier" | "destructor_name" => {
            node_text(node, source)
        }
        "pointer_declarator" | "reference_declarator" => {
            let decl = node.child_by_field_name("declarator")?;
            extract_declarator_name(decl, source)
        }
        _ => {
            // Try to find a nested function_declarator
            let cursor = &mut node.walk();
            for child in node.children(cursor) {
                if let Some(name) = extract_declarator_name(child, source) {
                    return Some(name);
                }
            }
            None
        }
    }
}

fn has_function_declarator(node: tree_sitter::Node) -> bool {
    if node.kind() == "function_declarator" {
        return true;
    }
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        if has_function_declarator(child) {
            return true;
        }
    }
    false
}

fn find_function_name_in_field(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    if node.kind() == "function_declarator" {
        let decl = node.child_by_field_name("declarator")?;
        return extract_declarator_name(decl, source);
    }
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        if let Some(name) = find_function_name_in_field(child, source) {
            return Some(name);
        }
    }
    None
}

fn parse_access_specifier(node: tree_sitter::Node, source: &[u8]) -> Visibility {
    let text = node.utf8_text(source).unwrap_or("");
    if text.contains("public") || text.contains("protected") {
        Visibility::Public
    } else {
        Visibility::Private
    }
}

fn extract_cpp_signature(node: tree_sitter::Node, source: &[u8]) -> String {
    let start = node.start_byte();
    let mut end = node.end_byte();
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        if child.kind() == "compound_statement" || child.kind() == "field_initializer_list" {
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

fn extract_cpp_doc(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut comments = Vec::new();
    let mut sibling = node.prev_sibling();
    while let Some(s) = sibling {
        if s.kind() == "comment" {
            let text = node_text(s, source)?;
            let cleaned = text
                .trim_start_matches("///")
                .trim_start_matches("//")
                .trim();
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

fn collect_cpp_calls(
    node: tree_sitter::Node,
    source: &[u8],
    current_fn: Option<&str>,
    edges: &mut Vec<CallEdge>,
) {
    let mut fn_name: Option<String> = current_fn.map(|s| s.to_string());

    if node.kind() == "function_definition"
        && let Some(decl) = node.child_by_field_name("declarator")
        && let Some(name) = extract_declarator_name(decl, source)
    {
        fn_name = Some(name);
    }

    if node.kind() == "call_expression"
        && let Some(ref caller) = fn_name
        && let Some(func_node) = node.child_by_field_name("function")
        && let Some(callee) = node_text(func_node, source)
    {
        edges.push((caller.clone(), callee));
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_cpp_calls(child, source, fn_name.as_deref(), edges);
    }
}

// ─── Identifier references ──────────────────────────────────────────

fn collect_cpp_ids(
    node: tree_sitter::Node,
    source: &[u8],
    target: &str,
    lines: &[&str],
    refs: &mut Vec<IdentifierRef>,
) {
    match node.kind() {
        "comment" | "string_literal" | "raw_string_literal" | "char_literal" => return,
        _ => {}
    }

    if (node.kind() == "identifier"
        || node.kind() == "type_identifier"
        || node.kind() == "field_identifier"
        || node.kind() == "namespace_identifier")
        && node_text(node, source).as_deref() == Some(target)
    {
        let line = node.start_position().row as u32 + 1;
        let context = lines
            .get(line as usize - 1)
            .unwrap_or(&"")
            .trim()
            .to_string();
        let kind = classify_cpp_ref(node);
        refs.push(IdentifierRef {
            line,
            context,
            kind,
        });
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_cpp_ids(child, source, target, lines, refs);
    }
}

fn classify_cpp_ref(node: tree_sitter::Node) -> RefKind {
    let parent = match node.parent() {
        Some(p) => p,
        None => return RefKind::Unknown,
    };
    match parent.kind() {
        "call_expression" => RefKind::Call,
        "preproc_include" => RefKind::Import,
        "type_identifier"
        | "type_descriptor"
        | "parameter_declaration"
        | "field_declaration"
        | "template_argument_list" => RefKind::TypeRef,
        "function_declarator" | "init_declarator" => {
            if parent.child_by_field_name("declarator").map(|n| n.id()) == Some(node.id()) {
                RefKind::Definition
            } else {
                RefKind::Unknown
            }
        }
        "class_specifier" | "struct_specifier" | "enum_specifier" => {
            if parent.child_by_field_name("name").map(|n| n.id()) == Some(node.id()) {
                RefKind::Definition
            } else {
                RefKind::TypeRef
            }
        }
        "field_expression" => {
            if let Some(gp) = parent.parent()
                && gp.kind() == "call_expression"
            {
                return RefKind::Call;
            }
            RefKind::FieldAccess
        }
        "qualified_identifier" | "scope_resolution" => {
            if let Some(gp) = parent.parent()
                && gp.kind() == "call_expression"
            {
                return RefKind::Call;
            }
            RefKind::TypeRef
        }
        "namespace_definition" => RefKind::Definition,
        _ => RefKind::Unknown,
    }
}
