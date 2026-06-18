use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::project::{FileKey, IndexStats, RootInfo};
use super::symbol::{Symbol, SymbolId};
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
    /// File mtime cache (for incremental updates) — nanosecond precision since UNIX_EPOCH.
    pub file_mtimes: HashMap<FileKey, u128>,
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
    /// Pre-computed identifier positions per file (keyed by relative path).
    /// Stored in a separate `idents.bin` file and loaded lazily by refs.
    #[serde(skip)]
    pub file_identifiers: HashMap<PathBuf, Vec<crate::parser::traits::IdentifierRef>>,
    /// Reverse index: identifier name → FileKeys containing it.
    #[serde(skip)]
    pub identifier_index: HashMap<String, Vec<FileKey>>,
    /// BLAKE3 hash of sorted root hashes, used as workspace cache key.
    pub workspace_hash: String,
    /// Index format version.
    pub version: u32,
    /// Timestamp when index was created/updated.
    pub indexed_at: u64,
    /// Pre-computed lowercase name + qualified_name for fast search.
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
            file_identifiers: HashMap::new(),
            identifier_index: HashMap::new(),
            workspace_hash,
            version: 4,
            indexed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            search_cache: HashMap::new(),
        }
    }

    /// Merge a single-root ProjectIndex into this workspace.
    /// Symbols and call edges use `label` (directory name) for display.
    /// File keys use `id` (hash) for internal deduplication.
    pub fn insert_from_project(&mut self, root_info: &RootInfo, project_index: &ProjectIndex) {
        let root_id = &root_info.id;
        let root_label = &root_info.label;

        // Insert symbols with label prefix for display
        for symbol in project_index.symbols.values() {
            let ws_symbol = self.remap_symbol(root_label, symbol);
            let ws_id = ws_symbol.id.clone();
            let lower_name = ws_symbol.name.to_lowercase();
            let lower_qname = ws_symbol.qualified_name.to_lowercase();
            self.search_cache
                .insert(ws_id.clone(), (lower_name, lower_qname));
            self.symbols.insert(ws_id.clone(), ws_symbol);

            // File keys use hash id for reliable dedup
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

        // Remap call edges: prefix with readable label
        for (rel_path, edges) in &project_index.file_call_edges {
            let file_key = FileKey::new(root_id, rel_path.clone());
            let ws_edges: Vec<CallEdge> = edges
                .iter()
                .map(|(caller, callee)| {
                    (
                        format!("[{}]{}", root_label, caller),
                        format!("[{}]{}", root_label, callee),
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

        // Merge identifier data
        for (rel_path, idents) in &project_index.file_identifiers {
            let mut seen_names = std::collections::HashSet::new();
            for ident in idents {
                if seen_names.insert(ident.name.as_str()) {
                    self.identifier_index
                        .entry(ident.name.clone())
                        .or_default()
                        .push(FileKey::new(root_id, rel_path.clone()));
                }
            }
            self.file_identifiers
                .insert(rel_path.clone(), idents.clone());
        }
    }

    /// Remove all data associated with a specific root.
    pub fn remove_root(&mut self, root_id: &str) {
        // Find the label for this root_id (SymbolId prefixes use label, not hash)
        let root_label = self
            .roots
            .iter()
            .find(|r| r.id == root_id)
            .map(|r| r.label.as_str())
            .unwrap_or("");

        // Remove symbols belonging to this root (match by label prefix)
        let sym_ids_to_remove: Vec<SymbolId> = self
            .symbols
            .iter()
            .filter(|(id, _)| !root_label.is_empty() && id.root_id() == root_label)
            .map(|(id, _)| id.clone())
            .collect();

        for id in &sym_ids_to_remove {
            self.search_cache.remove(id);
            self.symbols.remove(id);
        }

        // Collect paths for this root from all keyed collections (some files have
        // identifiers but no symbols, so we must gather from multiple sources).
        let root_file_paths: std::collections::HashSet<PathBuf> = self
            .file_symbols
            .keys()
            .filter(|k| k.root_id == root_id)
            .map(|k| k.path.clone())
            .chain(
                self.identifier_index
                    .values()
                    .flat_map(|v| v.iter())
                    .filter(|fk| fk.root_id == root_id)
                    .map(|fk| fk.path.clone()),
            )
            .collect();

        // Remove file-level data for this root (FileKey uses hash id)
        self.file_symbols.retain(|k, ids| {
            if k.root_id == root_id {
                false
            } else {
                ids.retain(|id| !root_label.is_empty() && id.root_id() != root_label);
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

        // Clean identifier data for this root
        self.file_identifiers
            .retain(|path, _| !root_file_paths.contains(path));
        self.identifier_index.retain(|_, files| {
            files.retain(|fk| fk.root_id != root_id);
            !files.is_empty()
        });

        // Remove root from roots list
        self.roots.retain(|r| r.id != root_id);
    }

    /// Resolve a FileKey to an absolute path on the filesystem.
    /// Matches by `root_id` (hash) first, falls back to `label`.
    pub fn resolve_absolute(&self, file_key: &FileKey) -> Option<PathBuf> {
        self.roots
            .iter()
            .find(|r| r.id == file_key.root_id || r.label == file_key.root_id)
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
    /// Prefixes SymbolId, parent, children with the root's readable label.
    fn remap_symbol(&self, root_label: &str, symbol: &Symbol) -> Symbol {
        Symbol {
            id: SymbolId::new_with_root(
                root_label,
                &symbol.file_path.to_string_lossy(),
                &symbol.qualified_name,
                &symbol.kind,
            ),
            name: symbol.name.clone(),
            qualified_name: format!("[{}]{}", root_label, symbol.qualified_name),
            kind: symbol.kind,
            file_path: symbol.file_path.clone(),
            span: symbol.span,
            signature: symbol.signature.clone(),
            doc_comment: symbol.doc_comment.clone(),
            visibility: symbol.visibility,
            parent: symbol
                .parent
                .as_ref()
                .map(|p| SymbolId(format!("[{}]{}", root_label, p.0))),
            children: symbol
                .children
                .iter()
                .map(|c| SymbolId(format!("[{}]{}", root_label, c.0)))
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

use super::{detect_language, kind_priority};

use crate::model::project::ProjectIndex;
