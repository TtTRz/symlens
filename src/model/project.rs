use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::symbol::{Symbol, SymbolId};
#[cfg(feature = "native")]
use crate::config::Config;
use crate::graph::call_graph::CallGraph;
use crate::parser::traits::{CallEdge, ImportInfo};

/// Key for per-file data in workspace mode.
/// In single-root mode, `root_id` is an empty string — the key behaves like a bare PathBuf.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct FileKey {
    /// Workspace root identifier (empty string for single-root backward compat).
    /// Derived from blake3(root_path)[..8].
    pub root_id: String,
    /// Relative file path from the project root.
    pub path: PathBuf,
}

impl FileKey {
    pub fn new(root_id: &str, path: PathBuf) -> Self {
        Self {
            root_id: root_id.to_string(),
            path,
        }
    }

    /// Convert to display string: "[root_id]path" or "path" (when root_id is empty).
    pub fn display(&self) -> String {
        if self.root_id.is_empty() {
            self.path.to_string_lossy().into_owned()
        } else {
            format!("[{}]{}", self.root_id, self.path.to_string_lossy())
        }
    }
}

impl fmt::Display for FileKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

/// Metadata for a workspace root directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootInfo {
    /// Absolute path to the project root.
    pub path: PathBuf,
    /// Short stable identifier derived from blake3(root_path)[..8].
    /// Used for internal deduplication and cache keys.
    pub id: String,
    /// Human-readable label derived from the directory name (e.g. "audio", "frontend").
    /// Used as prefix in SymbolId and display output instead of the hash.
    pub label: String,
    /// Full hash derived from blake3(root_path)[..16].
    /// Same as ProjectIndex::root_hash — used for per-root cache lookup.
    pub hash: String,
    /// Per-root configuration loaded from symlens.toml.
    #[cfg(feature = "native")]
    #[serde(skip)]
    pub config: Config,
}

impl RootInfo {
    /// Create a RootInfo from an absolute path, computing id, label, and hash via blake3.
    pub fn new(path: PathBuf) -> Self {
        let path_str = path.to_string_lossy();
        let full_hash = blake3::hash(path_str.as_bytes()).to_hex();
        let label = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| full_hash[..8].to_string());
        Self {
            path,
            id: full_hash[..8].to_string(),
            label,
            hash: full_hash[..16].to_string(),
            #[cfg(feature = "native")]
            config: Config::default(),
        }
    }

    /// Create a RootInfo with explicit config.
    #[cfg(feature = "native")]
    pub fn with_config(path: PathBuf, config: Config) -> Self {
        let path_str = path.to_string_lossy();
        let full_hash = blake3::hash(path_str.as_bytes()).to_hex();
        let label = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| full_hash[..8].to_string());
        Self {
            path,
            id: full_hash[..8].to_string(),
            label,
            hash: full_hash[..16].to_string(),
            config,
        }
    }
}

use std::fmt;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIndex {
    /// Project root path
    pub root: PathBuf,
    /// BLAKE3 hash of root path (used for cache key)
    pub root_hash: String,
    /// Symbol table: SymbolId → Symbol
    pub symbols: HashMap<SymbolId, Symbol>,
    /// File → symbol IDs in that file
    pub file_symbols: HashMap<PathBuf, Vec<SymbolId>>,
    /// File mtime cache (for incremental updates) — nanosecond precision since UNIX_EPOCH
    pub file_mtimes: HashMap<PathBuf, u128>,
    /// Index format version
    pub version: u32,
    /// Timestamp when index was created/updated
    pub indexed_at: u64,
    /// Call graph (caller → callee relationships)
    pub call_graph: Option<CallGraph>,
    /// Import name → files that import it (for refs v3)
    pub import_names: HashMap<String, Vec<PathBuf>>,
    /// File content hashes for reliable incremental indexing (blake3, first 16 hex chars)
    pub file_hashes: HashMap<PathBuf, String>,
    /// Call edges per file (for incremental call graph rebuilds)
    pub file_call_edges: HashMap<PathBuf, Vec<CallEdge>>,
    /// Imports per file (for incremental import rebuilds)
    pub file_imports: HashMap<PathBuf, Vec<ImportInfo>>,
    /// Pre-computed identifier positions per file.
    /// Stored in a separate `idents.bin` file and loaded lazily by refs.
    #[serde(skip)]
    pub file_identifiers: HashMap<PathBuf, Vec<crate::parser::traits::IdentifierRef>>,
    /// Reverse index: identifier name → files containing it.
    #[serde(skip)]
    pub identifier_index: HashMap<String, Vec<PathBuf>>,
    /// Number of files dropped due to `max_files` cap during the last index run.
    pub files_truncated: usize,
    /// Number of files that failed to read or parse during the last index run.
    pub files_failed: usize,
    /// Up to 50 paths (relative to root) of failed files from the last index run.
    pub failed_paths: Vec<PathBuf>,
    /// Pre-computed lowercase name + qualified_name for fast search
    search_cache: HashMap<SymbolId, (String, String)>,
}

/// Statistics about the index
#[derive(Debug, Default, Serialize)]
pub struct IndexStats {
    pub total_files: usize,
    pub total_symbols: usize,
    pub by_language: HashMap<String, usize>,
    pub by_kind: HashMap<String, usize>,
    pub files_truncated: usize,
    pub files_failed: usize,
    pub failed_paths: Vec<PathBuf>,
}

impl ProjectIndex {
    pub fn new(root: PathBuf) -> Self {
        let root_hash = {
            #[cfg(feature = "native")]
            {
                blake3::hash(root.to_string_lossy().as_bytes()).to_hex()[..16].to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                // Simple hash fallback for WASM builds
                format!("{:x}", root.to_string_lossy().len())
            }
        };

        Self {
            root,
            root_hash,
            symbols: HashMap::new(),
            file_symbols: HashMap::new(),
            file_mtimes: HashMap::new(),
            version: 4,
            indexed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            call_graph: None,
            import_names: HashMap::new(),
            file_hashes: HashMap::new(),
            file_call_edges: HashMap::new(),
            file_imports: HashMap::new(),
            file_identifiers: HashMap::new(),
            identifier_index: HashMap::new(),
            files_truncated: 0,
            files_failed: 0,
            failed_paths: Vec::new(),
            search_cache: HashMap::new(),
        }
    }

    /// Insert a symbol into the index.
    pub fn insert(&mut self, symbol: Symbol) {
        let file = symbol.file_path.clone();
        let id = symbol.id.clone();
        let lower_name = symbol.name.to_lowercase();
        let lower_qname = symbol.qualified_name.to_lowercase();
        self.search_cache
            .insert(id.clone(), (lower_name, lower_qname));
        self.symbols.insert(id.clone(), symbol);
        self.file_symbols.entry(file).or_default().push(id);
    }

    /// Remove all symbols from a file (for fine-grained incremental updates).
    /// Note: the current incremental re-index walks all files and rebuilds the index,
    /// so deleted files are naturally excluded. This method is kept for future
    /// partial-update scenarios where only changed files are re-indexed.
    #[allow(dead_code)]
    pub fn remove_file(&mut self, file: &PathBuf) {
        if let Some(ids) = self.file_symbols.remove(file) {
            for id in ids {
                self.search_cache.remove(&id);
                self.symbols.remove(&id);
            }
        }
        self.file_mtimes.remove(file);
        self.file_hashes.remove(file);
        self.file_call_edges.remove(file);
        self.file_imports.remove(file);
        // Clean import_names entries pointing to this file
        self.import_names.retain(|_, files| {
            files.retain(|f| f != file);
            !files.is_empty()
        });
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

    /// Search symbols by name (uses pre-computed lowercase cache for speed).
    pub fn search(&self, query: &str, limit: usize) -> Vec<&Symbol> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<&Symbol> = self
            .symbols
            .iter()
            .filter(|(id, s)| {
                if let Some((ln, lqn)) = self.search_cache.get(id) {
                    ln.contains(&query_lower)
                        || lqn.contains(&query_lower)
                        || s.signature
                            .as_ref()
                            .is_some_and(|sig| sig.to_lowercase().contains(&query_lower))
                        || s.doc_comment
                            .as_ref()
                            .is_some_and(|doc| doc.to_lowercase().contains(&query_lower))
                } else {
                    s.name.to_lowercase().contains(&query_lower)
                        || s.qualified_name.to_lowercase().contains(&query_lower)
                }
            })
            .map(|(_, s)| s)
            .collect();

        // Sort: exact name match first, then by kind priority, then by name
        results.sort_by(|a, b| {
            let a_exact = self
                .search_cache
                .get(&a.id)
                .is_some_and(|(ln, _)| *ln == query_lower);
            let b_exact = self
                .search_cache
                .get(&b.id)
                .is_some_and(|(ln, _)| *ln == query_lower);
            b_exact
                .cmp(&a_exact)
                .then_with(|| kind_priority(&a.kind).cmp(&kind_priority(&b.kind)))
                .then_with(|| a.name.cmp(&b.name))
        });

        results.truncate(limit);
        results
    }

    /// Rebuild the pre-computed lowercase search cache after deserialization.
    pub fn rebuild_search_cache(&mut self) {
        self.search_cache.clear();
        self.search_cache.reserve(self.symbols.len());
        for (id, sym) in &self.symbols {
            self.search_cache.insert(
                id.clone(),
                (sym.name.to_lowercase(), sym.qualified_name.to_lowercase()),
            );
        }
    }

    pub fn search_cache_is_empty(&self) -> bool {
        self.search_cache.is_empty()
    }

    /// Compute index statistics.
    pub fn stats(&self) -> IndexStats {
        let mut stats = IndexStats {
            total_files: self.file_symbols.len(),
            total_symbols: self.symbols.len(),
            files_truncated: self.files_truncated,
            files_failed: self.files_failed,
            failed_paths: self.failed_paths.clone(),
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

use super::{detect_language, kind_priority};
