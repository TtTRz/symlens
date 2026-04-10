use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::symbol::{Symbol, SymbolId, SymbolKind};
use crate::graph::call_graph::CallGraph;

/// The in-memory project index — core data structure.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectIndex {
    /// Project root path
    pub root: PathBuf,
    /// BLAKE3 hash of root path (used for cache key)
    pub root_hash: String,
    /// Symbol table: SymbolId → Symbol
    pub symbols: HashMap<SymbolId, Symbol>,
    /// File → symbol IDs in that file
    pub file_symbols: HashMap<PathBuf, Vec<SymbolId>>,
    /// File mtime cache (for incremental updates)
    pub file_mtimes: HashMap<PathBuf, u64>,
    /// Index format version
    pub version: u32,
    /// Timestamp when index was created/updated
    pub indexed_at: u64,
    /// Call graph (caller → callee relationships)
    pub call_graph: Option<CallGraph>,
    /// Import name → files that import it (for refs v3)
    pub import_names: HashMap<String, Vec<PathBuf>>,
}

/// Statistics about the index
#[derive(Debug, Default, Serialize)]
pub struct IndexStats {
    pub total_files: usize,
    pub total_symbols: usize,
    pub by_language: HashMap<String, usize>,
    pub by_kind: HashMap<String, usize>,
}

impl ProjectIndex {
    pub fn new(root: PathBuf) -> Self {
        let root_hash = blake3::hash(root.to_string_lossy().as_bytes())
            .to_hex()[..16]
            .to_string();

        Self {
            root,
            root_hash,
            symbols: HashMap::new(),
            file_symbols: HashMap::new(),
            file_mtimes: HashMap::new(),
            version: 1,
            indexed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            call_graph: None,
            import_names: HashMap::new(),
        }
    }

    /// Insert a symbol into the index.
    pub fn insert(&mut self, symbol: Symbol) {
        let file = symbol.file_path.clone();
        let id = symbol.id.clone();
        self.symbols.insert(id.clone(), symbol);
        self.file_symbols.entry(file).or_default().push(id);
    }

    /// Remove all symbols from a file (for incremental updates).
    pub fn remove_file(&mut self, file: &PathBuf) {
        if let Some(ids) = self.file_symbols.remove(file) {
            for id in ids {
                self.symbols.remove(&id);
            }
        }
        self.file_mtimes.remove(file);
    }

    /// Get a symbol by ID.
    pub fn get(&self, id: &SymbolId) -> Option<&Symbol> {
        self.symbols.get(id)
    }

    /// Get all symbols in a file.
    pub fn symbols_in_file(&self, file: &PathBuf) -> Vec<&Symbol> {
        self.file_symbols
            .get(file)
            .map(|ids| ids.iter().filter_map(|id| self.symbols.get(id)).collect())
            .unwrap_or_default()
    }

    /// Search symbols by name (simple substring match for MVP).
    pub fn search(&self, query: &str, limit: usize) -> Vec<&Symbol> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<&Symbol> = self
            .symbols
            .values()
            .filter(|s| {
                s.name.to_lowercase().contains(&query_lower)
                    || s.qualified_name.to_lowercase().contains(&query_lower)
                    || s.signature
                        .as_ref()
                        .map(|sig| sig.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || s.doc_comment
                        .as_ref()
                        .map(|doc| doc.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .collect();

        // Sort: exact name match first, then by kind priority, then by name
        results.sort_by(|a, b| {
            let a_exact = a.name.to_lowercase() == query_lower;
            let b_exact = b.name.to_lowercase() == query_lower;
            b_exact
                .cmp(&a_exact)
                .then_with(|| kind_priority(&a.kind).cmp(&kind_priority(&b.kind)))
                .then_with(|| a.name.cmp(&b.name))
        });

        results.truncate(limit);
        results
    }

    /// Compute index statistics.
    pub fn stats(&self) -> IndexStats {
        let mut stats = IndexStats {
            total_files: self.file_symbols.len(),
            total_symbols: self.symbols.len(),
            ..Default::default()
        };

        for (file, ids) in &self.file_symbols {
            let lang = detect_language(file);
            *stats.by_language.entry(lang).or_insert(0) += ids.len();
        }

        for symbol in self.symbols.values() {
            *stats
                .by_kind
                .entry(symbol.kind.as_str().to_string())
                .or_insert(0) += 1;
        }

        stats
    }
}

fn kind_priority(kind: &SymbolKind) -> u8 {
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

fn detect_language(path: &PathBuf) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust".into(),
        Some("ts") | Some("tsx") => "typescript".into(),
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
