use crate::model::symbol::*;
use crate::parser::traits::{CallEdge, IdentifierRef, ImportInfo, LanguageParser, RefKind};
use std::path::Path;

pub struct GoParser;

impl LanguageParser for GoParser {
    fn extensions(&self) -> &[&str] {
        &["go"]
    }

    fn extract_symbols(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<Symbol>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_go::LANGUAGE.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse {}", file_path.display()))?;

        let mut symbols = Vec::new();
        let file_str = file_path.to_string_lossy();
        extract_go_node(tree.root_node(), source, &file_str, file_path, &mut symbols);
        Ok(symbols)
    }

    fn extract_calls(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<CallEdge>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_go::LANGUAGE.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse {}", file_path.display()))?;

        let mut edges = Vec::new();
        collect_go_calls(tree.root_node(), source, None, &mut edges);
        Ok(edges)
    }

    fn find_identifiers(
        &self,
        source: &[u8],
        target_name: &str,
    ) -> anyhow::Result<Vec<IdentifierRef>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_go::LANGUAGE.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("parse failed"))?;

        let mut refs = Vec::new();
        let lines: Vec<&str> = std::str::from_utf8(source).unwrap_or("").lines().collect();
        collect_go_ids(tree.root_node(), source, target_name, &lines, &mut refs);
        Ok(refs)
    }

    fn extract_imports(&self, source: &[u8], _file_path: &Path) -> anyhow::Result<Vec<ImportInfo>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_go::LANGUAGE.into())?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("parse failed"))?;

        let mut imports = Vec::new();
        collect_go_imports(tree.root_node(), source, &mut imports);
        Ok(imports)
    }
}

fn extract_go_node(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    symbols: &mut Vec<Symbol>,
) {
    match node.kind() {
        "function_declaration" => {
            if let Some(sym) = extract_go_func(node, source, file_str, file_path) {
                symbols.push(sym);
            }
        }
        "method_declaration" => {
            if let Some(sym) = extract_go_method(node, source, file_str, file_path) {
                symbols.push(sym);
            }
        }
        "type_declaration" => {
            let cursor = &mut node.walk();
            for child in node.children(cursor) {
                if child.kind() == "type_spec"
                    && let Some(sym) = extract_go_type_spec(child, source, file_str, file_path) {
                        symbols.push(sym);
                    }
            }
        }
        "const_declaration" | "var_declaration" => {
            extract_go_vars(node, source, file_str, file_path, symbols);
        }
        _ => {}
    }

    // Recurse into source_file
    if node.kind() == "source_file" {
        let cursor = &mut node.walk();
        for child in node.children(cursor) {
            extract_go_node(child, source, file_str, file_path, symbols);
        }
    }
}

fn extract_go_func(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    let vis = if name
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
    {
        Visibility::Public
    } else {
        Visibility::Private
    };
    let sig = extract_go_signature(node, source);
    let doc = extract_go_doc(node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &SymbolKind::Function),
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Function,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(sig),
        doc_comment: doc,
        visibility: vis,
        parent: None,
        children: vec![],
    })
}

fn extract_go_method(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;

    // Get receiver type
    let receiver_type = node
        .child_by_field_name("receiver")
        .and_then(|r| {
            // parameter_list → parameter_declaration → type
            let cursor = &mut r.walk();
            for child in r.children(cursor) {
                if child.kind() == "parameter_declaration"
                    && let Some(type_node) = child.child_by_field_name("type") {
                        return node_text(type_node, source);
                    }
            }
            None
        })
        .map(|t| t.trim_start_matches('*').to_string());

    let qualified = match &receiver_type {
        Some(recv) => format!("{}::{}", recv, name),
        None => name.clone(),
    };

    let vis = if name
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
    {
        Visibility::Public
    } else {
        Visibility::Private
    };

    let sig = extract_go_signature(node, source);
    let doc = extract_go_doc(node, source);

    Some(Symbol {
        id: SymbolId::new(file_str, &qualified, &SymbolKind::Method),
        name,
        qualified_name: qualified,
        kind: SymbolKind::Method,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(sig),
        doc_comment: doc,
        visibility: vis,
        parent: receiver_type.map(|r| SymbolId::new(file_str, &r, &SymbolKind::Struct)),
        children: vec![],
    })
}

fn extract_go_type_spec(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    let type_node = node.child_by_field_name("type")?;

    let kind = match type_node.kind() {
        "struct_type" => SymbolKind::Struct,
        "interface_type" => SymbolKind::Interface,
        _ => SymbolKind::TypeAlias,
    };

    let vis = if name
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
    {
        Visibility::Public
    } else {
        Visibility::Private
    };

    let doc = extract_go_doc(node.parent().unwrap_or(node), source);

    Some(Symbol {
        id: SymbolId::new(file_str, &name, &kind),
        name: name.clone(),
        qualified_name: name,
        kind,
        file_path: file_path.to_path_buf(),
        span: node_span(node),
        signature: Some(format!(
            "type {} {}",
            node_text(name_node, source)?,
            type_node.kind().replace("_type", "")
        )),
        doc_comment: doc,
        visibility: vis,
        parent: None,
        children: vec![],
    })
}

fn extract_go_vars(
    node: tree_sitter::Node,
    source: &[u8],
    file_str: &str,
    file_path: &Path,
    symbols: &mut Vec<Symbol>,
) {
    let is_const = node.kind() == "const_declaration";
    let kind = if is_const {
        SymbolKind::Constant
    } else {
        SymbolKind::Variable
    };

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        if (child.kind() == "const_spec" || child.kind() == "var_spec")
            && let Some(name_node) = child.child_by_field_name("name")
                && let Some(name) = node_text(name_node, source) {
                    let vis = if name
                        .chars()
                        .next()
                        .map(|c| c.is_uppercase())
                        .unwrap_or(false)
                    {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    };

                    let first_line = node
                        .utf8_text(source)
                        .ok()
                        .and_then(|t| t.lines().next().map(|l| l.trim().to_string()));

                    symbols.push(Symbol {
                        id: SymbolId::new(file_str, &name, &kind),
                        name: name.clone(),
                        qualified_name: name,
                        kind,
                        file_path: file_path.to_path_buf(),
                        span: node_span(child),
                        signature: first_line,
                        doc_comment: extract_go_doc(node, source),
                        visibility: vis,
                        parent: None,
                        children: vec![],
                    });
                }
    }
}

fn extract_go_signature(node: tree_sitter::Node, source: &[u8]) -> String {
    let start = node.start_byte();
    let mut end = node.end_byte();
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        if child.kind() == "block" {
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

fn extract_go_doc(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
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

fn collect_go_calls(
    node: tree_sitter::Node,
    source: &[u8],
    current_fn: Option<&str>,
    edges: &mut Vec<CallEdge>,
) {
    let mut fn_name = current_fn;

    if (node.kind() == "function_declaration" || node.kind() == "method_declaration")
        && let Some(name_node) = node.child_by_field_name("name")
            && let Some(name) = node_text(name_node, source) {
                fn_name = Some(Box::leak(name.into_boxed_str()));
            }

    if node.kind() == "call_expression"
        && let Some(caller) = fn_name
            && let Some(func_node) = node.child_by_field_name("function")
                && let Some(callee) = node_text(func_node, source) {
                    edges.push((caller.to_string(), callee));
                }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_go_calls(child, source, fn_name, edges);
    }
}

// ─── Identifier references ──────────────────────────────────────────

fn collect_go_ids(
    node: tree_sitter::Node,
    source: &[u8],
    target: &str,
    lines: &[&str],
    refs: &mut Vec<IdentifierRef>,
) {
    match node.kind() {
        "comment" | "interpreted_string_literal" | "raw_string_literal" | "rune_literal" => return,
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
        let kind = classify_go_ref(node);
        refs.push(IdentifierRef {
            line,
            context,
            kind,
        });
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_go_ids(child, source, target, lines, refs);
    }
}

fn classify_go_ref(node: tree_sitter::Node) -> RefKind {
    let parent = match node.parent() {
        Some(p) => p,
        None => return RefKind::Unknown,
    };
    match parent.kind() {
        "call_expression" => RefKind::Call,
        "import_spec" | "import_declaration" => RefKind::Import,
        "type_identifier" | "type_spec" | "parameter_declaration" | "field_declaration" => {
            RefKind::TypeRef
        }
        "function_declaration" | "method_declaration" => {
            if parent.child_by_field_name("name").map(|n| n.id()) == Some(node.id()) {
                RefKind::Definition
            } else {
                RefKind::Unknown
            }
        }
        "selector_expression" => {
            if let Some(gp) = parent.parent()
                && gp.kind() == "call_expression" {
                    return RefKind::Call;
                }
            RefKind::FieldAccess
        }
        "composite_literal" => RefKind::Constructor,
        _ => RefKind::Unknown,
    }
}

// ─── Import extraction ──────────────────────────────────────────────

fn collect_go_imports(node: tree_sitter::Node, source: &[u8], imports: &mut Vec<ImportInfo>) {
    if node.kind() == "import_spec"
        && let Some(path_node) = node.child_by_field_name("path")
            && let Some(path_text) = node_text(path_node, source) {
                let cleaned = path_text.trim_matches('"');
                let pkg_name = cleaned.rsplit('/').next().unwrap_or(cleaned).to_string();
                imports.push(ImportInfo {
                    module_path: cleaned.to_string(),
                    names: vec![pkg_name],
                });
            }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        collect_go_imports(child, source, imports);
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
