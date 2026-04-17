use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::project::{FileKey, IndexStats, RootInfo};
use super::symbol::{Symbol, SymbolId, SymbolKind};
use crate::graph::call_graph::CallGraph;
use crate::parser::traits::{CallEdge, ImportInfo};

/// The in-memory workspace index — a unified index spanning multiple project roots.
/// Each root contributes its own symbols, call edges, and imports, all merged into
/// a single searchable structure with `[root_id]` prefixes for disambiguation.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceIndex {
    /// Ordered list of workspace roots.
    pub roots: Vec<RootInfo>,
    /// Symbol table: SymbolId → Symbol.
    /// SymbolId includes `[root_id]` prefix in workspace mode.
    pub symbols: HashMap<SymbolId, Symbol>,
    /// FileKey → symbol IDs in that file.
    pub file_symbols: HashMap<FileKey, Vec<SymbolId>>,
    /// File mtime cache (for incremental updates).
    pub file_mtimes: HashMap<FileKey, u64>,
    /// File content hashes for reliable incremental indexing.
    pub file_hashes: HashMap<FileKey, String>,
    /// Call graph (caller → callee relationships).
    /// Node names include `[root_id]` prefix for cross-root uniqueness.
    pub call_graph: Option<CallGraph>,
    /// Import name → files that import it (for refs v3).
    pub import_names: HashMap<String, Vec<FileKey>>,
    /// Call edges per file (for incremental call graph rebuilds).
    pub file_call_edges: HashMap<FileKey, Vec<CallEdge>>,
    /// Imports per file (for incremental import rebuilds).
    pub file_imports: HashMap<FileKey, Vec<ImportInfo>>,
    /// BLAKE3 hash of sorted root hashes, used as workspace cache key.
    pub workspace_hash: String,
    /// Index format version.
    pub version: u32,
    /// Timestamp when index was created/updated.
    pub indexed_at: u64,
    /// Pre-computed lowercase name + qualified_name for fast search.
    #[serde(skip)]
    search_cache: HashMap<SymbolId, (String, String)>,
}

impl WorkspaceIndex {
    /// Create an empty workspace index for the given roots.
    pub fn new(roots: &[RootInfo]) -> Self {
        let workspace_hash = compute_workspace_hash(roots);
        Self {
            roots: roots.to_vec(),
            symbols: HashMap::new(),
            file_symbols: HashMap::new(),
            file_mtimes: HashMap::new(),
            file_hashes: HashMap::new(),
            call_graph: None,
            import_names: HashMap::new(),
            file_call_edges: HashMap::new(),
            file_imports: HashMap::new(),
            workspace_hash,
            version: 2,
            indexed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            search_cache: HashMap::new(),
        }
    }

    /// Merge a single-root ProjectIndex into this workspace.
    /// All SymbolIds and file keys are prefixed with the root's id.
    /// Call edges are remapped to include root-scoped qualified names.
    pub fn insert_from_project(&mut self, root_info: &RootInfo, project_index: &ProjectIndex) {
        let root_id = &root_info.id;

        // Insert symbols with root_id prefix
        for symbol in project_index.symbols.values() {
            let ws_symbol = self.remap_symbol(root_id, symbol);
            let ws_id = ws_symbol.id.clone();
            let lower_name = ws_symbol.name.to_lowercase();
            let lower_qname = ws_symbol.qualified_name.to_lowercase();
            self.search_cache
                .insert(ws_id.clone(), (lower_name, lower_qname));
            self.symbols.insert(ws_id.clone(), ws_symbol);

            // Update file_symbols
            let file_key = FileKey::new(root_id, symbol.file_path.clone());
            self.file_symbols.entry(file_key).or_default().push(ws_id);
        }

        // Copy per-file metadata
        for (rel_path, mtime) in &project_index.file_mtimes {
            let file_key = FileKey::new(root_id, rel_path.clone());
            self.file_mtimes.insert(file_key, *mtime);
        }
        for (rel_path, hash) in &project_index.file_hashes {
            let file_key = FileKey::new(root_id, rel_path.clone());
            self.file_hashes.insert(file_key, hash.clone());
        }

        // Remap call edges: prefix qualified names with root_id
        for (rel_path, edges) in &project_index.file_call_edges {
            let file_key = FileKey::new(root_id, rel_path.clone());
            let ws_edges: Vec<CallEdge> = edges
                .iter()
                .map(|(caller, callee)| {
                    (
                        format!("[{}]{}", root_id, caller),
                        format!("[{}]{}", root_id, callee),
                    )
                })
                .collect();
            self.file_call_edges.insert(file_key, ws_edges);
        }

        // Remap imports
        for (rel_path, imports) in &project_index.file_imports {
            let file_key = FileKey::new(root_id, rel_path.clone());
            self.file_imports.insert(file_key, imports.clone());
        }
        for (name, files) in &project_index.import_names {
            let entry = self.import_names.entry(name.clone()).or_default();
            for rel_path in files {
                entry.push(FileKey::new(root_id, rel_path.clone()));
            }
        }
    }

    /// Remove all data associated with a specific root.
    pub fn remove_root(&mut self, root_id: &str) {
        // Remove symbols belonging to this root
        let sym_ids_to_remove: Vec<SymbolId> = self
            .symbols
            .iter()
            .filter(|(id, _)| id.root_id() == root_id)
            .map(|(id, _)| id.clone())
            .collect();

        for id in &sym_ids_to_remove {
            self.search_cache.remove(id);
            self.symbols.remove(id);
        }

        // Remove file-level data for this root
        self.file_symbols.retain(|k, ids| {
            if k.root_id == root_id {
                false
            } else {
                ids.retain(|id| id.root_id() != root_id);
                !ids.is_empty()
            }
        });
        self.file_mtimes.retain(|k, _| k.root_id != root_id);
        self.file_hashes.retain(|k, _| k.root_id != root_id);
        self.file_call_edges.retain(|k, _| k.root_id != root_id);
        self.file_imports.retain(|k, _| k.root_id != root_id);

        // Clean import_names
        self.import_names.retain(|_, files| {
            files.retain(|f| f.root_id != root_id);
            !files.is_empty()
        });

        // Remove root from roots list
        self.roots.retain(|r| r.id != root_id);
    }

    /// Resolve a FileKey to an absolute path on the filesystem.
    pub fn resolve_absolute(&self, file_key: &FileKey) -> Option<PathBuf> {
        self.roots
            .iter()
            .find(|r| r.id == file_key.root_id)
            .map(|r| r.path.join(&file_key.path))
    }

    /// Get a symbol by ID.
    pub fn get(&self, id: &SymbolId) -> Option<&Symbol> {
        self.symbols.get(id)
    }

    /// Get all symbols in a file.
    pub fn symbols_in_file(&self, file_key: &FileKey) -> Vec<&Symbol> {
        self.file_symbols
            .get(file_key)
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

    /// Compute index statistics.
    pub fn stats(&self) -> IndexStats {
        let mut stats = IndexStats {
            total_files: self.file_symbols.len(),
            total_symbols: self.symbols.len(),
            ..Default::default()
        };

        for (file_key, ids) in &self.file_symbols {
            let lang = detect_language(&file_key.path);
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

    /// Build the call graph from all collected file-level call edges.
    pub fn build_call_graph(&mut self) {
        let mut all_edges: Vec<CallEdge> = Vec::new();
        for edges in self.file_call_edges.values() {
            all_edges.extend(edges.clone());
        }
        if !all_edges.is_empty() {
            self.call_graph = Some(CallGraph::build(&all_edges));
        }
    }

    /// Remap a Symbol from a single-root ProjectIndex to workspace form.
    /// Prefixes SymbolId, parent, children with root_id.
    fn remap_symbol(&self, root_id: &str, symbol: &Symbol) -> Symbol {
        Symbol {
            id: SymbolId::new_with_root(
                root_id,
                &symbol.file_path.to_string_lossy(),
                &symbol.qualified_name,
                &symbol.kind,
            ),
            name: symbol.name.clone(),
            qualified_name: format!("[{}]{}", root_id, symbol.qualified_name),
            kind: symbol.kind,
            file_path: symbol.file_path.clone(),
            span: symbol.span,
            signature: symbol.signature.clone(),
            doc_comment: symbol.doc_comment.clone(),
            visibility: symbol.visibility,
            // Prepend root_id prefix to parent/child SymbolIds.
            // Since SymbolId is a newtype wrapper around String, we directly
            // prefix the inner string — this preserves the original kind tag.
            parent: symbol
                .parent
                .as_ref()
                .map(|p| SymbolId(format!("[{}]{}", root_id, p.0))),
            children: symbol
                .children
                .iter()
                .map(|c| SymbolId(format!("[{}]{}", root_id, c.0)))
                .collect(),
        }
    }
}

/// Compute a stable hash for a set of workspace roots.
/// Sorts root hashes before hashing to ensure deterministic results.
pub fn compute_workspace_hash(roots: &[RootInfo]) -> String {
    let mut hashes: Vec<&str> = roots.iter().map(|r| r.hash.as_str()).collect();
    hashes.sort();
    let concatenated: String = hashes.join("");
    blake3::hash(concatenated.as_bytes()).to_hex()[..16].to_string()
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

fn detect_language(path: &std::path::Path) -> String {
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

use crate::model::project::ProjectIndex;
