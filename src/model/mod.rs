pub mod project;
pub mod symbol;
pub mod workspace;

use crate::model::symbol::SymbolKind;

/// Priority for symbol kind in search result ordering.
pub fn kind_priority(kind: &SymbolKind) -> u8 {
    match kind {
        SymbolKind::Function | SymbolKind::Method => 0,
        SymbolKind::Struct | SymbolKind::Class => 1,
        SymbolKind::Interface => 2,
        SymbolKind::Enum => 3,
        SymbolKind::Constant => 4,
        SymbolKind::TypeAlias => 5,
        SymbolKind::Macro => 6,
        _ => 7,
    }
}

/// Detect programming language from a file extension.
pub fn detect_language(path: &std::path::Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust".into(),
        Some("ts") | Some("tsx") | Some("mts") | Some("cts") => "typescript".into(),
        Some("js") | Some("jsx") => "javascript".into(),
        Some("py") => "python".into(),
        Some("swift") => "swift".into(),
        Some("go") => "go".into(),
        Some("c") | Some("h") => "c".into(),
        Some("cpp") | Some("hpp") | Some("cc") | Some("cxx") => "cpp".into(),
        Some("java") => "java".into(),
        Some(ext) => ext.to_string(),
        None => "unknown".into(),
    }
}
