use crate::model::symbol::*;
use crate::parser::traits::{CallEdge, IdentifierRef, ImportInfo, LanguageParser, RefKind};
use std::path::Path;

pub struct DartParser;

impl LanguageParser for DartParser {
    fn extensions(&self) -> &[&str] {
        &["dart"]
    }

    fn extract_symbols(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<Symbol>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_dart::LANGUAGE.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse {}", file_path.display()))?;

        let mut symbols = Vec::new();
        let file_str = file_path.to_string_lossy();
        extract_dart_node(
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
        parser.set_language(&tree_sitter_dart::LANGUAGE.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse {}", file_path.display()))?;

        let mut edges = Vec::new();
        collect_dart_calls(tree.root_node(), source, None, &mut edges);
        Ok(edges)
    }

    fn find_identifiers(
        &self,
        source: &[u8],
        target_name: &str,
    ) -> anyhow::Result<Vec<IdentifierRef>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_dart::LANGUAGE.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("parse failed"))?;

        let mut refs = Vec::new();
        let lines: Vec<&str> = std::str::from_utf8(source).unwrap_or("").lines().collect();
        collect_dart_ids(tree.root_node(), source, target_name, &lines, &mut refs);
        Ok(refs)
    }

    fn extract_imports(&self, source: &[u8], _file_path: &Path) -> anyhow::Result<Vec<ImportInfo>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_dart::LANGUAGE.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("parse failed"))?;

        let mut imports = Vec::new();
        collect_dart_imports(tree.root_node(), source, &mut imports);
        Ok(imports)
    }
}

// ─── Symbol extraction ──────────────────────────────────────────────

fn extract_dart_node(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    class_name: Option<&str>,
    symbols: &mut Vec<Symbol>,
) {
    match node.kind() {
        "class_declaration" => {
            if let Some(sym) = extract_dart_class(node, source, file_str, file_path) {
                let cname = sym.name.clone();
                symbols.push(sym);
                // Recurse into class body
                if let Some(body) = find_child_by_kind(node, "class_body") {
                    let cursor = &mut body.walk();
                    for child in body.children(cursor) {
                        extract_dart_node(
                            child,
                            source,
                            file_str,
                            file_path,
                            Some(&cname),
                            symbols,
                        );
                    }
                }
                return; // Don't recurse again
            }
        }
        "mixin_declaration" => {
            if let Some(sym) = extract_dart_mixin(node, source, file_str, file_path) {
                let mname = sym.name.clone();
                symbols.push(sym);
                if let Some(body) = find_child_by_kind(node, "class_body") {
                    let cursor = &mut body.walk();
                    for child in body.children(cursor) {
                        extract_dart_node(
                            child,
                            source,
                            file_str,
                            file_path,
                            Some(&mname),
                            symbols,
                        );
                    }
                }
                return;
            }
        }
        "extension_declaration" => {
            if let Some(sym) = extract_dart_extension(node, source, file_str, file_path) {
                let ename = sym.name.clone();
                symbols.push(sym);
                if let Some(body) = find_child_by_kind(node, "extension_body") {
                    let cursor = &mut body.walk();
                    for child in body.children(cursor) {
                        extract_dart_node(
                            child,
                            source,
                            file_str,
                            file_path,
                            Some(&ename),
                            symbols,
                        );
                    }
                }
                return;
            }
        }
        "enum_declaration" => {
            if let Some(sym) = extract_dart_enum(node, source, file_str, file_path) {
                symbols.push(sym);
            }
        }
        "type_alias" => {
            if let Some(sym) = extract_dart_typedef(node, source, file_str, file_path) {
                symbols.push(sym);
            }
        }
        // Top-level or class member: function/method declaration
        "declaration" => {
            // A declaration node wraps function_signature, getter_signature, etc.
            if let Some(sym) =
                extract_dart_declaration(node, source, file_str, file_path, class_name)
            {
                symbols.push(sym);
                return;
            }
        }
        "class_member" => {
            // class_member wraps declaration inside class body
            let cursor = &mut node.walk();
            for child in node.children(cursor) {
                extract_dart_node(child, source, file_str, file_path, class_name, symbols);
            }
            return;
        }
        _ => {}
    }

    // Recurse for source_file and other containers
    if class_name.is_none() {
        let cursor = &mut node.walk();
        let children: Vec<_> = node.children(cursor).collect();
        let mut i = 0;
        while i < children.len() {
            let child = children[i];
            match child.kind() {
                // Top-level function: function_signature followed by function_body
                "function_signature" | "method_signature" => {
                    if let Some(sym) =
                        extract_dart_toplevel_func(child, &children, i, source, file_str, file_path)
                    {
                        symbols.push(sym);
                    }
                }
                "getter_signature" => {
                    if let Some(sym) = extract_dart_toplevel_accessor(
                        child, &children, i, source, file_str, file_path, "get",
                    ) {
                        symbols.push(sym);
                    }
                }
                "setter_signature" => {
                    if let Some(sym) = extract_dart_toplevel_accessor(
                        child, &children, i, source, file_str, file_path, "set",
                    ) {
                        symbols.push(sym);
                    }
                }
                "static_final_declaration_list" | "initialized_identifier_list" => {
                    // Top-level const/final/var
                    if let Some(sym) = extract_dart_toplevel_var(child, source, file_str, file_path)
                    {
                        symbols.push(sym);
                    }
                }
                _ => {
                    extract_dart_node(child, source, file_str, file_path, None, symbols);
                }
            }
            i += 1;
        }
    }
}

fn extract_dart_class(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name = find_child_text_by_kind(node, "identifier", source)?;
    let doc = extract_dart_doc(node, source);
    let sig = extract_dart_class_signature(node, source);

    let is_abstract = node
        .utf8_text(source)
        .ok()
        .map(|t| t.starts_with("abstract"))
        .unwrap_or(false);

    let kind = if is_abstract {
        SymbolKind::Interface
    } else {
        SymbolKind::Class
    };

    Some(Symbol {
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
        children: vec![],
    })
}

fn extract_dart_mixin(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name = find_child_text_by_kind(node, "identifier", source)?;
    let doc = extract_dart_doc(node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Interface),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Interface,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(format!("mixin {}", node_text_first_line(node, source))),
        doc_comment: doc,
        visibility: Visibility::Public,
        parent: None,
        children: vec![],
    })
}

fn extract_dart_extension(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    // extension Foo on Bar { ... }
    let name = find_child_text_by_kind(node, "identifier", source)
        .or_else(|| find_child_text_by_kind(node, "extension_type_name", source))
        .unwrap_or_else(|| "<anonymous>".to_string());
    let doc = extract_dart_doc(node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Class),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Class,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(node_text_first_line(node, source)),
        doc_comment: doc,
        visibility: Visibility::Public,
        parent: None,
        children: vec![],
    })
}

fn extract_dart_enum(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name = find_child_text_by_kind(node, "identifier", source)?;
    let doc = extract_dart_doc(node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Enum),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Enum,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(format!("enum {}", node_text_first_line(node, source))),
        doc_comment: doc,
        visibility: Visibility::Public,
        parent: None,
        children: vec![],
    })
}

fn extract_dart_typedef(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name = find_child_text_by_kind(node, "type_identifier", source)
        .or_else(|| find_child_text_by_kind(node, "identifier", source))?;
    let doc = extract_dart_doc(node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::TypeAlias),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::TypeAlias,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(node_text_first_line(node, source)),
        doc_comment: doc,
        visibility: Visibility::Public,
        parent: None,
        children: vec![],
    })
}

fn extract_dart_declaration(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    class_name: Option<&str>,
) -> Option<Symbol> {
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        match child.kind() {
            "function_signature" => {
                return extract_dart_func_sig(child, node, source, file_str, file_path, class_name);
            }
            "method_signature" => {
                return extract_dart_func_sig(child, node, source, file_str, file_path, class_name);
            }
            "getter_signature" => {
                return extract_dart_accessor(
                    child, node, source, file_str, file_path, class_name, "get",
                );
            }
            "setter_signature" => {
                return extract_dart_accessor(
                    child, node, source, file_str, file_path, class_name, "set",
                );
            }
            "constructor_signature" | "constant_constructor_signature" => {
                return extract_dart_constructor(
                    child, node, source, file_str, file_path, class_name,
                );
            }
            "factory_constructor_signature" | "redirecting_factory_constructor_signature" => {
                return extract_dart_constructor(
                    child, node, source, file_str, file_path, class_name,
                );
            }
            "initialized_variable_definition" | "static_final_declaration_list" => {
                return extract_dart_field(child, node, source, file_str, file_path, class_name);
            }
            _ => {}
        }
    }
    None
}

fn extract_dart_func_sig(
    sig_node: tree_sitter::Node,
    decl_node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    class_name: Option<&str>,
) -> Option<Symbol> {
    let name = find_child_text_by_kind(sig_node, "identifier", source)?;
    let is_private = name.starts_with('_');
    let doc = extract_dart_doc(decl_node, source);

    let (kind, qualified) = match class_name {
        Some(cls) => (SymbolKind::Method, format!("{}::{}", cls, name)),
        None => (SymbolKind::Function, name.clone()),
    };

    let sig = extract_dart_sig_text(sig_node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &qualified, &kind),
        name,
        qualified_name: qualified,
        kind,
        file_path: file_path.to_path_buf(),
        span: node_span(decl_node),
        signature: Some(sig),
        doc_comment: doc,
        visibility: if is_private {
            Visibility::Private
        } else {
            Visibility::Public
        },
        parent: class_name.map(|c| SymbolId::new(file_str, c, &SymbolKind::Class)),
        children: vec![],
    })
}

fn extract_dart_accessor(
    sig_node: tree_sitter::Node,
    decl_node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    class_name: Option<&str>,
    accessor_kind: &str,
) -> Option<Symbol> {
    let name = find_child_text_by_kind(sig_node, "identifier", source)?;
    let is_private = name.starts_with('_');
    let doc = extract_dart_doc(decl_node, source);

    let qualified = match class_name {
        Some(cls) => format!("{}::{}", cls, name),
        None => name.clone(),
    };

    let sig = format!(
        "{} {}",
        accessor_kind,
        extract_dart_sig_text(sig_node, source)
    );

    Some(Symbol {
        id: SymbolId::new(file_str, &qualified, &SymbolKind::Method),
        name,
        qualified_name: qualified,
        kind: SymbolKind::Method,
        file_path: file_path.to_path_buf(),
        span: node_span(decl_node),
        signature: Some(sig),
        doc_comment: doc,
        visibility: if is_private {
            Visibility::Private
        } else {
            Visibility::Public
        },
        parent: class_name.map(|c| SymbolId::new(file_str, c, &SymbolKind::Class)),
        children: vec![],
    })
}

fn extract_dart_constructor(
    sig_node: tree_sitter::Node,
    decl_node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    class_name: Option<&str>,
) -> Option<Symbol> {
    let cls = class_name.unwrap_or("Unknown");

    // Named constructor: Class.name(...)
    let name_part = find_child_text_by_kind(sig_node, "identifier", source);
    let name = match &name_part {
        Some(n) if n != cls => format!("{}.{}", cls, n),
        _ => cls.to_string(),
    };

    let doc = extract_dart_doc(decl_node, source);
    let sig = extract_dart_sig_text(sig_node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Method),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Method,
        file_path: file_path.to_path_buf(),
        span: node_span(decl_node),
        signature: Some(sig),
        doc_comment: doc,
        visibility: Visibility::Public,
        parent: Some(SymbolId::new(file_str, cls, &SymbolKind::Class)),
        children: vec![],
    })
}

fn extract_dart_field(
    field_node: tree_sitter::Node,
    decl_node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    class_name: Option<&str>,
) -> Option<Symbol> {
    let name = find_child_text_by_kind(field_node, "identifier", source)?;
    let is_private = name.starts_with('_');
    let doc = extract_dart_doc(decl_node, source);

    let is_static = decl_node
        .utf8_text(source)
        .ok()
        .map(|t| t.contains("static") || t.contains("const"))
        .unwrap_or(false);

    let kind = if is_static && class_name.is_none() {
        SymbolKind::Constant
    } else {
        SymbolKind::Variable
    };

    let qualified = match class_name {
        Some(cls) => format!("{}::{}", cls, name),
        None => name.clone(),
    };

    Some(Symbol {
        id: SymbolId::new(file_str, &qualified, &kind),
        name,
        qualified_name: qualified,
        kind,
        file_path: file_path.to_path_buf(),
        span: node_span(decl_node),
        signature: Some(node_text_first_line(decl_node, source)),
        doc_comment: doc,
        visibility: if is_private {
            Visibility::Private
        } else {
            Visibility::Public
        },
        parent: class_name.map(|c| SymbolId::new(file_str, c, &SymbolKind::Class)),
        children: vec![],
    })
}

// ─── Top-level declarations (source_file level) ─────────────────────

fn extract_dart_toplevel_func(
    sig_node: tree_sitter::Node,
    siblings: &[tree_sitter::Node],
    idx: usize,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name = find_child_text_by_kind(sig_node, "identifier", source)?;
    let is_private = name.starts_with('_');
    let doc = extract_dart_doc(sig_node, source);
    let sig = extract_dart_sig_text(sig_node, source);

    // Span: from sig_node start to function_body end (if exists)
    let end_line = if idx + 1 < siblings.len() && siblings[idx + 1].kind() == "function_body" {
        siblings[idx + 1].end_position().row as u32 + 1
    } else {
        sig_node.end_position().row as u32 + 1
    };

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Function),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Function,
        file_path: file_path.to_path_buf(),
        span: Span {
            start_line: sig_node.start_position().row as u32 + 1,
            end_line,
            start_col: sig_node.start_position().column as u32,
            end_col: sig_node.end_position().column as u32,
        },
        signature: Some(sig),
        doc_comment: doc,
        visibility: if is_private {
            Visibility::Private
        } else {
            Visibility::Public
        },
        parent: None,
        children: vec![],
    })
}

fn extract_dart_toplevel_accessor(
    sig_node: tree_sitter::Node,
    siblings: &[tree_sitter::Node],
    idx: usize,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    accessor_kind: &str,
) -> Option<Symbol> {
    let name = find_child_text_by_kind(sig_node, "identifier", source)?;
    let is_private = name.starts_with('_');
    let doc = extract_dart_doc(sig_node, source);
    let sig = format!(
        "{} {}",
        accessor_kind,
        extract_dart_sig_text(sig_node, source)
    );

    let end_line = if idx + 1 < siblings.len()
        && (siblings[idx + 1].kind() == "function_body"
            || siblings[idx + 1].kind() == "function_expression_body")
    {
        siblings[idx + 1].end_position().row as u32 + 1
    } else {
        sig_node.end_position().row as u32 + 1
    };

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Function),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Function,
        file_path: file_path.to_path_buf(),
        span: Span {
            start_line: sig_node.start_position().row as u32 + 1,
            end_line,
            start_col: sig_node.start_position().column as u32,
            end_col: sig_node.end_position().column as u32,
        },
        signature: Some(sig),
        doc_comment: doc,
        visibility: if is_private {
            Visibility::Private
        } else {
            Visibility::Public
        },
        parent: None,
        children: vec![],
    })
}

fn extract_dart_toplevel_var(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name = find_child_text_by_kind(node, "identifier", source)?;
    let is_private = name.starts_with('_');
    let doc = extract_dart_doc(node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Constant),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Constant,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(node_text_first_line(node, source)),
        doc_comment: doc,
        visibility: if is_private {
            Visibility::Private
        } else {
            Visibility::Public
        },
        parent: None,
        children: vec![],
    })
}

// ─── Call extraction ────────────────────────────────────────────────

fn collect_dart_calls(
    node: tree_sitter::Node,
    source: &[u8],
    current_fn: Option<&str>,
    edges: &mut Vec<CallEdge>,
) {
    let mut fn_name = current_fn;

    // Track current function/method scope
    if (node.kind() == "function_signature" || node.kind() == "method_signature")
        && let Some(name) = find_child_text_by_kind(node, "identifier", source)
    {
        fn_name = Some(Box::leak(name.into_boxed_str()));
    }

    // Detect function/constructor calls:
    // Pattern 1: identifier followed by argument_part → function call
    // Pattern 2: type_identifier followed by argument_part → constructor call
    if (node.kind() == "identifier" || node.kind() == "type_identifier")
        && node.parent().map(|p| p.kind()) != Some("function_signature")
        && node.parent().map(|p| p.kind()) != Some("method_signature")
        && node.parent().map(|p| p.kind()) != Some("getter_signature")
        && node.parent().map(|p| p.kind()) != Some("setter_signature")
        && let Some(next) = node.next_sibling()
        && (next.kind() == "selector"
            || next.kind() == "argument_part"
            || next.kind() == "arguments")
        && let Some(caller) = fn_name
        && let Some(callee) = node_text(node, source)
    {
        edges.push((caller.to_string(), callee));
    }

    // Pattern 3: unconditional_assignable_selector(.identifier) followed by argument_part
    if node.kind() == "unconditional_assignable_selector"
        && let Some(id) = find_child_text_by_kind(node, "identifier", source)
        && let Some(next) = node.next_sibling()
        && (next.kind() == "argument_part" || next.kind() == "arguments")
        && let Some(caller) = fn_name
    {
        edges.push((caller.to_string(), id));
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_dart_calls(child, source, fn_name, edges);
    }
}

// ─── Identifier references ──────────────────────────────────────────

fn collect_dart_ids(
    node: tree_sitter::Node,
    source: &[u8],
    target: &str,
    lines: &[&str],
    refs: &mut Vec<IdentifierRef>,
) {
    // Skip string literals and comments
    match node.kind() {
        "comment"
        | "documentation_block_comment"
        | "string_literal"
        | "string_literal_single_quotes"
        | "string_literal_double_quotes"
        | "string_literal_single_quotes_multiple"
        | "string_literal_double_quotes_multiple"
        | "raw_string_literal_single_quotes"
        | "raw_string_literal_double_quotes"
        | "template_substitution" => return,
        _ => {}
    }

    if (node.kind() == "identifier" || node.kind() == "type_identifier")
        && node_text(node, source).as_deref() == Some(target)
    {
        let line = node.start_position().row as u32 + 1;
        let context = lines
            .get(line as usize - 1)
            .unwrap_or(&"")
            .trim()
            .to_string();
        let kind = classify_dart_ref(node);
        refs.push(IdentifierRef {
            line,
            context,
            kind,
        });
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_dart_ids(child, source, target, lines, refs);
    }
}

fn classify_dart_ref(node: tree_sitter::Node) -> RefKind {
    let parent = match node.parent() {
        Some(p) => p,
        None => return RefKind::Unknown,
    };

    match parent.kind() {
        "import_specification" | "library_import" | "import_or_export" => RefKind::Import,
        "function_signature" | "method_signature" | "getter_signature" | "setter_signature" => {
            // Check if this identifier is the function name
            if find_child_by_kind(parent, "identifier").map(|n| n.id()) == Some(node.id()) {
                RefKind::Definition
            } else {
                RefKind::TypeRef
            }
        }
        "class_declaration"
        | "mixin_declaration"
        | "enum_declaration"
        | "extension_declaration" => {
            if find_child_by_kind(parent, "identifier").map(|n| n.id()) == Some(node.id()) {
                RefKind::Definition
            } else {
                RefKind::TypeRef
            }
        }
        "type_identifier" | "type_arguments" | "superclass" | "mixins" | "formal_parameter"
        | "typed_identifier" => RefKind::TypeRef,
        "constructor_invocation" | "const_object_expression" => RefKind::Constructor,
        "unconditional_assignable_selector" | "conditional_assignable_selector" => {
            // method.call() — check grandparent
            if let Some(gp) = parent.parent()
                && (gp.kind() == "postfix_expression" || gp.kind() == "argument_part")
            {
                return RefKind::Call;
            }
            RefKind::FieldAccess
        }
        "selector" => {
            if let Some(next) = node.next_sibling()
                && (next.kind() == "argument_part" || next.kind() == "arguments")
            {
                return RefKind::Call;
            }
            RefKind::FieldAccess
        }
        _ => {
            // Check if next sibling is argument_part (direct call)
            if let Some(next) = node.next_sibling()
                && (next.kind() == "argument_part" || next.kind() == "arguments")
            {
                return RefKind::Call;
            }
            RefKind::Unknown
        }
    }
}

// ─── Import extraction ──────────────────────────────────────────────

fn collect_dart_imports(node: tree_sitter::Node, source: &[u8], imports: &mut Vec<ImportInfo>) {
    if node.kind() == "import_or_export" || node.kind() == "library_import" {
        // Find the URI
        if let Some(uri_node) = find_child_by_kind(node, "import_specification")
            .and_then(|spec| find_child_by_kind(spec, "configurable_uri"))
            .and_then(|cu| find_child_by_kind(cu, "uri"))
            && let Some(uri_text) = node_text(uri_node, source)
        {
            let cleaned = uri_text.trim_matches('\'').trim_matches('"');

            // Extract "show" names if present
            let mut names = Vec::new();
            let cursor = &mut node.walk();
            for child in node.children(cursor) {
                if child.kind() == "combinator"
                    && let Ok(text) = child.utf8_text(source)
                    && text.starts_with("show")
                {
                    // Parse "show Foo, Bar, Baz"
                    let parts = text.trim_start_matches("show").trim();
                    for name in parts.split(',') {
                        let n = name.trim();
                        if !n.is_empty() {
                            names.push(n.to_string());
                        }
                    }
                }
            }

            // If no show clause, use the last segment of the import path
            if names.is_empty() {
                let pkg_name = cleaned
                    .rsplit('/')
                    .next()
                    .unwrap_or(cleaned)
                    .trim_end_matches(".dart");
                // Also check for package:name/name.dart pattern
                if cleaned.starts_with("package:") {
                    let pkg = cleaned
                        .trim_start_matches("package:")
                        .split('/')
                        .next()
                        .unwrap_or(cleaned);
                    names.push(pkg.to_string());
                }
                names.push(pkg_name.to_string());
            }

            imports.push(ImportInfo {
                module_path: cleaned.to_string(),
                names,
            });
        }
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_dart_imports(child, source, imports);
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

fn extract_dart_sig_text(sig_node: tree_sitter::Node, source: &[u8]) -> String {
    sig_node
        .utf8_text(source)
        .ok()
        .map(|t| t.lines().map(|l| l.trim()).collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}

fn extract_dart_class_signature(node: tree_sitter::Node, source: &[u8]) -> String {
    let start = node.start_byte();
    let mut end = node.end_byte();
    // Stop before class body
    if let Some(body) = find_child_by_kind(node, "class_body") {
        end = body.start_byte();
    }
    let sig = &source[start..end];
    String::from_utf8_lossy(sig)
        .trim()
        .lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_dart_doc(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut comments = Vec::new();
    let mut sibling = node.prev_sibling();
    while let Some(s) = sibling {
        match s.kind() {
            "comment" => {
                let text = node_text(s, source)?;
                let cleaned = text
                    .trim_start_matches("///")
                    .trim_start_matches("//")
                    .trim();
                comments.push(cleaned.to_string());
            }
            "documentation_block_comment" => {
                if let Some(text) = node_text(s, source) {
                    let cleaned = text
                        .trim_start_matches("/**")
                        .trim_end_matches("*/")
                        .lines()
                        .map(|l| l.trim().trim_start_matches('*').trim())
                        .filter(|l| !l.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n");
                    comments.push(cleaned);
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
