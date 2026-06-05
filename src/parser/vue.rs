use crate::model::symbol::*;
use crate::parser::traits::{CallEdge, IdentifierRef, ImportInfo, LanguageParser, ParsedOutput};
use crate::parser::typescript::TypeScriptParser;
use std::path::Path;

pub struct VueParser;

/// A `<script>` block extracted from a .vue SFC file.
struct ScriptBlock {
    /// The script body content (between `<script...>` and `</script>`).
    content: String,
    /// Number of `'\n'` characters before the script body in the .vue file.
    /// Used to adjust parsed line numbers back to original .vue positions.
    line_offset: u32,
    /// Whether this is a `<script setup>` block.
    _is_setup: bool,
}

impl LanguageParser for VueParser {
    fn extensions(&self) -> &[&str] {
        &["vue"]
    }

    fn language(&self) -> tree_sitter::Language {
        // Vue files are parsed by extracting <script> blocks and delegating
        // to the TypeScript parser. Return TS as the nominal language.
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn extract_symbols(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<Symbol>> {
        let blocks = extract_script_blocks(source);
        let ts_parser = TypeScriptParser;
        let mut all_symbols = Vec::new();

        for block in &blocks {
            let mut symbols = ts_parser.extract_symbols(block.content.as_bytes(), file_path)?;
            adjust_symbol_lines(&mut symbols, block.line_offset);
            all_symbols.extend(symbols);
        }

        Ok(all_symbols)
    }

    fn extract_calls(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<CallEdge>> {
        let blocks = extract_script_blocks(source);
        let ts_parser = TypeScriptParser;
        let mut all_edges = Vec::new();

        for block in &blocks {
            let edges = ts_parser.extract_calls(block.content.as_bytes(), file_path)?;
            all_edges.extend(edges);
        }

        Ok(all_edges)
    }

    fn find_identifiers(
        &self,
        source: &[u8],
        target_name: &str,
    ) -> anyhow::Result<Vec<IdentifierRef>> {
        let blocks = extract_script_blocks(source);
        let ts_parser = TypeScriptParser;
        let mut all_refs = Vec::new();

        for block in &blocks {
            let mut refs = ts_parser.find_identifiers(block.content.as_bytes(), target_name)?;
            for r in &mut refs {
                r.line += block.line_offset;
            }
            all_refs.extend(refs);
        }

        Ok(all_refs)
    }

    fn extract_imports(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<ImportInfo>> {
        let blocks = extract_script_blocks(source);
        let ts_parser = TypeScriptParser;
        let mut all_imports = Vec::new();

        for block in &blocks {
            let imports = ts_parser.extract_imports(block.content.as_bytes(), file_path)?;
            all_imports.extend(imports);
        }

        Ok(all_imports)
    }

    /// Override `extract_all` to avoid parsing the raw .vue with tree-sitter-TS
    /// (which would hit ERROR nodes from HTML markup). Instead, extract script
    /// blocks first and delegate each one to the TypeScript parser.
    fn extract_all(&self, source: &[u8], file_path: &Path) -> anyhow::Result<ParsedOutput> {
        let blocks = extract_script_blocks(source);
        if blocks.is_empty() {
            return Ok(ParsedOutput::default());
        }

        let ts_parser = TypeScriptParser;
        let mut all_symbols = Vec::new();
        let mut all_call_edges = Vec::new();
        let mut all_imports = Vec::new();
        let mut all_identifiers = Vec::new();

        for block in &blocks {
            let output = ts_parser.extract_all(block.content.as_bytes(), file_path)?;

            let mut symbols = output.symbols;
            adjust_symbol_lines(&mut symbols, block.line_offset);
            all_symbols.extend(symbols);

            all_call_edges.extend(output.call_edges);
            all_imports.extend(output.imports);
            all_identifiers.extend(output.identifiers);
        }

        Ok(ParsedOutput {
            symbols: all_symbols,
            call_edges: all_call_edges,
            imports: all_imports,
            identifiers: all_identifiers,
        })
    }
}

/// Shift symbol line numbers by the given offset so they point into the
/// original .vue file instead of the extracted script block.
fn adjust_symbol_lines(symbols: &mut [Symbol], offset: u32) {
    for sym in symbols.iter_mut() {
        sym.span.start_line += offset;
        sym.span.end_line += offset;
    }
}

/// Extract all `<script>` / `<script setup>` blocks from a .vue SFC.
///
/// Uses simple string scanning instead of regex to avoid an extra dependency.
fn extract_script_blocks(source: &[u8]) -> Vec<ScriptBlock> {
    let text = match std::str::from_utf8(source) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut blocks = Vec::new();
    let mut pos = 0;

    while pos < text.len() {
        // Find "<script"
        let tag_start = match text[pos..].find("<script") {
            Some(i) => pos + i,
            None => break,
        };

        // Make sure it's actually a tag (next char is whitespace or '>')
        let after = &text[tag_start + 7..];
        let next_ch = after.chars().next();
        if next_ch.is_some_and(|c| c != '>' && !c.is_whitespace()) {
            // e.g. "<scriptx" — not a real script tag, skip
            pos = tag_start + 7;
            continue;
        }

        // Find the closing '>' of the opening tag
        let tag_end = match text[tag_start..].find('>') {
            Some(i) => tag_start + i,
            None => break,
        };

        // Attributes between "<script" and ">"
        let attrs = &text[tag_start + 7..tag_end];
        let is_setup = attrs.contains("setup");

        // Find "</script>"
        let content_start = tag_end + 1;
        let close_tag = match text[content_start..].find("</script>") {
            Some(i) => content_start + i,
            None => break,
        };

        let content = text[content_start..close_tag].to_string();

        // Line offset = number of '\n' characters before content_start.
        // If content_start is at line N (1-indexed) in the .vue file,
        // then the first line of the extracted script is .vue line N,
        // and offset = N - 1.
        let line_offset = text[..content_start].chars().filter(|&c| c == '\n').count() as u32;

        blocks.push(ScriptBlock {
            content,
            line_offset,
            _is_setup: is_setup,
        });

        pos = close_tag + 9; // after "</script>"
    }

    blocks
}
