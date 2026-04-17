pub mod blame;
pub mod callers;
pub mod completions;
pub mod diff;
pub mod doctor;
pub mod export;
pub mod graph;
pub mod index;
pub mod init;
pub mod lines;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod outline;
pub mod refs;
pub mod search;
pub mod setup;
pub mod stats;
pub mod symbol;
pub mod watch;

use crate::config::WorkspaceConfig;
use crate::graph::call_graph::CallGraph;
use crate::index::storage;
use crate::model::project::{FileKey, ProjectIndex, RootInfo};
use crate::model::symbol::{Symbol, SymbolId};
use crate::model::workspace::WorkspaceIndex;
use std::path::{Path, PathBuf};

/// Resolve project root: use explicit --root if provided, otherwise auto-detect via .git.
pub fn resolve_root(explicit: Option<&str>) -> anyhow::Result<PathBuf> {
    if let Some(root) = explicit {
        let p = PathBuf::from(root).canonicalize()?;
        return Ok(p);
    }
    let cwd = std::env::current_dir()?;
    Ok(crate::index::storage::find_project_root(&cwd).unwrap_or(cwd))
}

/// Unified index provider that abstracts single-root and workspace modes.
/// Commands interact with this instead of directly with ProjectIndex.
pub enum IndexProvider {
    Single { root: PathBuf, index: ProjectIndex },
    Workspace { index: WorkspaceIndex },
}

impl IndexProvider {
    /// Load an index for the given root/workspace configuration.
    /// If workspace_flag is true or symlens.workspace.toml exists, use workspace mode.
    /// Otherwise, fall back to single-root mode.
    pub fn load(root_override: Option<&str>, workspace_flag: bool) -> anyhow::Result<Self> {
        let root = resolve_root(root_override)?;

        // Check for workspace config
        let ws_config = WorkspaceConfig::load(&root);

        if workspace_flag || ws_config.is_some() {
            // Workspace mode
            let config = ws_config.ok_or_else(|| {
                anyhow::anyhow!(
                    "Workspace mode requested but no symlens.workspace.toml found in {}",
                    root.display()
                )
            })?;

            let root_paths = config.resolve_roots(&root);
            if root_paths.is_empty() {
                anyhow::bail!("No valid roots found in symlens.workspace.toml");
            }

            let roots: Vec<RootInfo> = root_paths
                .iter()
                .map(|p| {
                    let mut info = RootInfo::new(p.clone());
                    // Load per-root config if it exists
                    info.config = crate::config::Config::load(p);
                    // Apply workspace defaults for fields not set per-root
                    if info.config.max_files == 100_000
                        && config.workspace.defaults.max_files != 100_000
                    {
                        info.config.max_files = config.workspace.defaults.max_files;
                    }
                    if info.config.ignore.is_empty() && !config.workspace.defaults.ignore.is_empty()
                    {
                        info.config.ignore = config.workspace.defaults.ignore.clone();
                    }
                    if info.config.languages.is_empty()
                        && !config.workspace.defaults.languages.is_empty()
                    {
                        info.config.languages = config.workspace.defaults.languages.clone();
                    }
                    info
                })
                .collect();

            let index = storage::load_workspace(&roots)?.ok_or_else(|| {
                anyhow::anyhow!("No workspace index found. Run `symlens index` first.")
            })?;

            Ok(IndexProvider::Workspace { index })
        } else {
            // Single-root mode
            let index = storage::load(&root)?
                .ok_or_else(|| anyhow::anyhow!("No index found. Run `symlens index` first."))?;

            Ok(IndexProvider::Single { root, index })
        }
    }

    /// Resolve a FileKey (root_id + relative path) to an absolute filesystem path.
    pub fn resolve_absolute(&self, root_id: &str, rel_path: &Path) -> PathBuf {
        match self {
            IndexProvider::Single { root, .. } => {
                // In single-root mode, root_id is always empty
                root.join(rel_path)
            }
            IndexProvider::Workspace { index } => index
                .resolve_absolute(&FileKey::new(root_id, rel_path.to_path_buf()))
                .unwrap_or_else(|| rel_path.to_path_buf()),
        }
    }

    /// Get a symbol by ID.
    pub fn get(&self, id: &SymbolId) -> Option<&Symbol> {
        match self {
            IndexProvider::Single { index, .. } => index.get(id),
            IndexProvider::Workspace { index } => index.get(id),
        }
    }

    /// Get the call graph, if available.
    pub fn call_graph(&self) -> Option<&CallGraph> {
        match self {
            IndexProvider::Single { index, .. } => index.call_graph.as_ref(),
            IndexProvider::Workspace { index } => index.call_graph.as_ref(),
        }
    }

    /// Search symbols by name.
    pub fn search(&self, query: &str, limit: usize) -> Vec<&Symbol> {
        match self {
            IndexProvider::Single { index, .. } => index.search(query, limit),
            IndexProvider::Workspace { index } => index.search(query, limit),
        }
    }

    /// Compute index statistics.
    pub fn stats(&self) -> crate::model::project::IndexStats {
        match self {
            IndexProvider::Single { index, .. } => index.stats(),
            IndexProvider::Workspace { index } => index.stats(),
        }
    }

    /// Get the project root path (single-root mode).
    /// Returns None in workspace mode.
    pub fn single_root(&self) -> Option<&Path> {
        match self {
            IndexProvider::Single { root, .. } => Some(root),
            IndexProvider::Workspace { .. } => None,
        }
    }

    /// Get all file keys (for iteration).
    pub fn file_keys(&self) -> Vec<FileKey> {
        match self {
            IndexProvider::Single { index, .. } => index
                .file_symbols
                .keys()
                .map(|p| FileKey::new("", p.clone()))
                .collect(),
            IndexProvider::Workspace { index } => index.file_symbols.keys().cloned().collect(),
        }
    }

    /// Get symbols in a specific file.
    pub fn symbols_in_file(&self, file_key: &FileKey) -> Vec<&Symbol> {
        match self {
            IndexProvider::Single { index, .. } => index.symbols_in_file(&file_key.path),
            IndexProvider::Workspace { index } => index.symbols_in_file(file_key),
        }
    }

    /// Get files that import a given name (for refs narrowing).
    pub fn import_names_for(&self, name: &str) -> Vec<FileKey> {
        match self {
            IndexProvider::Single { index, .. } => index
                .import_names
                .get(name)
                .map(|paths| paths.iter().map(|p| FileKey::new("", p.clone())).collect())
                .unwrap_or_default(),
            IndexProvider::Workspace { index } => {
                index.import_names.get(name).cloned().unwrap_or_default()
            }
        }
    }

    /// Find a symbol by name (exact or partial match on qualified_name or name).
    pub fn find_symbol(&self, name: &str) -> Option<&Symbol> {
        match self {
            IndexProvider::Single { index, .. } => index
                .symbols
                .values()
                .find(|s| s.id.0 == name || s.qualified_name == name || s.name == name),
            IndexProvider::Workspace { index } => index.symbols.values().find(|s| {
                s.name == name
                    || s.qualified_name.ends_with(name)
                    || s.qualified_name == format!("[{}]", name) // edge case
            }),
        }
    }

    /// Get all symbols.
    pub fn symbols(&self) -> Vec<&Symbol> {
        match self {
            IndexProvider::Single { index, .. } => index.symbols.values().collect(),
            IndexProvider::Workspace { index } => index.symbols.values().collect(),
        }
    }

    /// Get the number of files in the index.
    pub fn file_count(&self) -> usize {
        match self {
            IndexProvider::Single { index, .. } => index.file_symbols.len(),
            IndexProvider::Workspace { index } => index.file_symbols.len(),
        }
    }

    /// Whether this is a workspace provider.
    pub fn is_workspace(&self) -> bool {
        matches!(self, IndexProvider::Workspace { .. })
    }

    /// Get the index format version.
    pub fn version(&self) -> u32 {
        match self {
            IndexProvider::Single { index, .. } => index.version,
            IndexProvider::Workspace { index } => index.version,
        }
    }

    /// Get the timestamp when the index was created/updated.
    pub fn indexed_at(&self) -> u64 {
        match self {
            IndexProvider::Single { index, .. } => index.indexed_at,
            IndexProvider::Workspace { index } => index.indexed_at,
        }
    }

    /// Get workspace roots (workspace mode) or single root info.
    /// Returns a list of `(id, path, hash)` tuples.
    pub fn roots(&self) -> Vec<(&str, &Path, &str)> {
        match self {
            IndexProvider::Single { root, index } => {
                vec![("", root.as_path(), index.root_hash.as_str())]
            }
            IndexProvider::Workspace { index } => index
                .roots
                .iter()
                .map(|r| (r.id.as_str(), r.path.as_path(), r.hash.as_str()))
                .collect(),
        }
    }
}
