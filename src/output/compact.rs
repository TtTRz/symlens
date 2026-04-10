use crate::model::symbol::Symbol;

/// Format search results in compact mode (default, optimized for AI).
pub fn format_search_results(symbols: &[(&Symbol, f32)]) -> String {
    let mut out = String::new();
    for (sym, _score) in symbols {
        out.push_str(&format!("{} [{}]\n", sym.id, sym.span));
        if let Some(ref sig) = sym.signature {
            out.push_str(&format!("  {}\n", sig));
        }
        if let Some(ref doc) = sym.doc_comment {
            if let Some(first_line) = doc.lines().next() {
                if !first_line.is_empty() {
                    out.push_str(&format!("  /// {}\n", first_line));
                }
            }
        }
        out.push('\n');
    }
    out.push_str(&format!("{} results", symbols.len()));
    out
}
