use crate::model::project::{FileKey, ProjectIndex, RootInfo};
use crate::model::workspace::WorkspaceIndex;
use crate::parser::traits::IdentifierRef;
use crate::search::bm25::SearchEngine;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const INDEX_FILE: &str = "index.bin";
const META_FILE: &str = "meta.json";
const SEARCH_DIR: &str = "search";
const IDENT_FILE: &str = "idents.bin";
const CURRENT_INDEX_VERSION: u32 = 3;

/// Get the cache directory for a project.
pub fn cache_dir(root_hash: &str) -> PathBuf {
    let home = dirs_or_default();
    home.join(".symlens").join("indexes").join(root_hash)
}

/// Save a project index to disk, including tantivy search index.
pub fn save(index: &ProjectIndex) -> anyhow::Result<PathBuf> {
    let dir = cache_dir(&index.root_hash);
    fs::create_dir_all(&dir)?;

    // Save index as bincode (identifier fields are #[serde(skip)])
    let index_path = dir.join(INDEX_FILE);
    let encoded = bincode::serde::encode_to_vec(index, bincode::config::standard())?;
    let tmp_index = dir.join("index.bin.tmp");
    fs::write(&tmp_index, encoded)?;
    fs::rename(&tmp_index, &index_path)?;

    // Save identifier data to separate file
    if let Err(e) = save_identifiers(&dir, &index.file_identifiers, &index.identifier_index) {
        eprintln!("warning: failed to save identifier data: {e}");
    }

    // Reuse existing tantivy index if available (index_symbols clears + re-adds)
    let search_dir = dir.join(SEARCH_DIR);
    let engine = if search_dir.exists() {
        SearchEngine::open(&search_dir).or_else(|_| SearchEngine::create(&search_dir))?
    } else {
        SearchEngine::create(&search_dir)?
    };
    let symbols: Vec<&_> = index.symbols.values().collect();
    engine.index_symbols(&symbols)?;

    // Save metadata as JSON
    let meta = serde_json::json!({
        "root": index.root.to_string_lossy(),
        "version": index.version,
        "indexed_at": index.indexed_at,
        "files": index.file_symbols.len(),
        "symbols": index.symbols.len(),
    });
    let tmp_meta = dir.join("meta.json.tmp");
    fs::write(&tmp_meta, serde_json::to_string_pretty(&meta)?)?;
    fs::rename(&tmp_meta, dir.join(META_FILE))?;

    Ok(dir)
}

/// Load a project index from disk.
pub fn load(root: &Path) -> anyhow::Result<Option<ProjectIndex>> {
    let root_hash = blake3::hash(root.to_string_lossy().as_bytes()).to_hex()[..16].to_string();

    let index_path = cache_dir(&root_hash).join(INDEX_FILE);

    if !index_path.exists() {
        return Ok(None);
    }

    let data = fs::read(&index_path)?;
    let (mut index, _): (ProjectIndex, _) =
        bincode::serde::decode_from_slice(&data, bincode::config::standard())?;

    if index.version < CURRENT_INDEX_VERSION {
        return Ok(None);
    }

    // Rebuild call graph indexes after deserialization.
    // v2+ indices serialize name_to_idx and short_name_idx, so only the
    // petgraph DiGraph (which cannot impl serde) needs rebuilding.
    // v1 indices need the full rebuild including name_to_idx.
    if let Some(ref mut cg) = index.call_graph {
        if index.version >= 2 {
            cg.rebuild_digraph();
        } else {
            cg.rebuild_index();
        }
    }

    // v2+ indices serialize search_cache; only v1 needs rebuild.
    if index.version < 2 {
        index.rebuild_search_cache();
    }

    Ok(Some(index))
}

/// Open the tantivy search engine for a project.
pub fn open_search(root: &Path) -> anyhow::Result<Option<SearchEngine>> {
    let root_hash = blake3::hash(root.to_string_lossy().as_bytes()).to_hex()[..16].to_string();

    let search_dir = cache_dir(&root_hash).join(SEARCH_DIR);
    if !search_dir.exists() {
        return Ok(None);
    }

    let engine = SearchEngine::open(&search_dir)?;
    Ok(Some(engine))
}

type FileIdentMap = HashMap<PathBuf, Vec<IdentifierRef>>;
type IdentIndexMap = HashMap<String, Vec<PathBuf>>;
type WsIdentIndexMap = HashMap<String, Vec<FileKey>>;

/// Identifier data stored separately for lazy loading.
#[derive(serde::Serialize, serde::Deserialize)]
struct IdentifierData {
    file_identifiers: FileIdentMap,
    identifier_index: IdentIndexMap,
}

/// Workspace identifier data stored separately for lazy loading.
#[derive(serde::Serialize, serde::Deserialize)]
struct WorkspaceIdentifierData {
    file_identifiers: FileIdentMap,
    identifier_index: WsIdentIndexMap,
}

fn save_identifiers(
    dir: &Path,
    file_identifiers: &FileIdentMap,
    identifier_index: &IdentIndexMap,
) -> anyhow::Result<()> {
    let data = IdentifierData {
        file_identifiers: file_identifiers.clone(),
        identifier_index: identifier_index.clone(),
    };
    let encoded = bincode::serde::encode_to_vec(&data, bincode::config::standard())?;
    let tmp = dir.join("idents.bin.tmp");
    fs::write(&tmp, encoded)?;
    fs::rename(&tmp, dir.join(IDENT_FILE))?;
    Ok(())
}

/// Load identifier data for a single-root project.
pub fn load_identifiers(root: &Path) -> anyhow::Result<Option<(FileIdentMap, IdentIndexMap)>> {
    let root_hash = blake3::hash(root.to_string_lossy().as_bytes()).to_hex()[..16].to_string();
    let ident_path = cache_dir(&root_hash).join(IDENT_FILE);
    if !ident_path.exists() {
        return Ok(None);
    }
    let data = fs::read(&ident_path)?;
    let (id, _): (IdentifierData, _) =
        bincode::serde::decode_from_slice(&data, bincode::config::standard())?;
    Ok(Some((id.file_identifiers, id.identifier_index)))
}

/// Load identifier data for a workspace.
pub fn load_workspace_identifiers(
    ws_hash: &str,
) -> anyhow::Result<Option<(FileIdentMap, WsIdentIndexMap)>> {
    let ident_path = workspace_cache_dir(ws_hash).join(IDENT_FILE);
    if !ident_path.exists() {
        return Ok(None);
    }
    let data = fs::read(&ident_path)?;
    let (id, _): (WorkspaceIdentifierData, _) =
        bincode::serde::decode_from_slice(&data, bincode::config::standard())?;
    Ok(Some((id.file_identifiers, id.identifier_index)))
}

/// Find the project root (walk up looking for .git).
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn dirs_or_default() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

// ---------------------------------------------------------------------------
// Workspace storage
// ---------------------------------------------------------------------------

/// Get the cache directory for a workspace.
/// Workspace caches are stored under `ws_{hash}` to distinguish from per-root caches.
pub fn workspace_cache_dir(workspace_hash: &str) -> PathBuf {
    let home = dirs_or_default();
    home.join(".symlens")
        .join("indexes")
        .join(format!("ws_{}", workspace_hash))
}

/// Compute a stable hash for a set of workspace roots.
/// Sorts root hashes before hashing to ensure deterministic results.
pub fn compute_workspace_hash(roots: &[RootInfo]) -> String {
    let mut hashes: Vec<&str> = roots.iter().map(|r| r.hash.as_str()).collect();
    hashes.sort();
    let mut hasher = blake3::Hasher::new();
    for h in &hashes {
        hasher.update(h.as_bytes());
    }
    hasher.finalize().to_hex()[..16].to_string()
}

/// Save a workspace index to disk, including tantivy search index.
pub fn save_workspace(index: &WorkspaceIndex) -> anyhow::Result<PathBuf> {
    let dir = workspace_cache_dir(&index.workspace_hash);
    fs::create_dir_all(&dir)?;

    // Save index as bincode (identifier fields are #[serde(skip)])
    let index_path = dir.join(INDEX_FILE);
    let encoded = bincode::serde::encode_to_vec(index, bincode::config::standard())?;
    let tmp_index = dir.join("index.bin.tmp");
    fs::write(&tmp_index, encoded)?;
    fs::rename(&tmp_index, &index_path)?;

    // Save identifier data to separate file
    let ws_id = WorkspaceIdentifierData {
        file_identifiers: index.file_identifiers.clone(),
        identifier_index: index.identifier_index.clone(),
    };
    match bincode::serde::encode_to_vec(&ws_id, bincode::config::standard()) {
        Ok(encoded) => {
            let tmp = dir.join("idents.bin.tmp");
            if let Err(e) =
                fs::write(&tmp, encoded).and_then(|_| fs::rename(&tmp, dir.join(IDENT_FILE)))
            {
                eprintln!("warning: failed to save workspace identifier data: {e}");
            }
        }
        Err(e) => eprintln!("warning: failed to encode workspace identifier data: {e}"),
    }

    // Reuse existing tantivy index if available
    let search_dir = dir.join(SEARCH_DIR);
    let engine = if search_dir.exists() {
        SearchEngine::open(&search_dir).or_else(|_| SearchEngine::create(&search_dir))?
    } else {
        SearchEngine::create(&search_dir)?
    };
    let symbols: Vec<&_> = index.symbols.values().collect();
    engine.index_symbols(&symbols)?;

    // Save metadata as JSON
    let root_paths: Vec<String> = index
        .roots
        .iter()
        .map(|r| r.path.to_string_lossy().into_owned())
        .collect();
    let meta = serde_json::json!({
        "roots": root_paths,
        "workspace_hash": index.workspace_hash,
        "version": index.version,
        "indexed_at": index.indexed_at,
        "files": index.file_symbols.len(),
        "symbols": index.symbols.len(),
    });
    let tmp_meta = dir.join("meta.json.tmp");
    fs::write(&tmp_meta, serde_json::to_string_pretty(&meta)?)?;
    fs::rename(&tmp_meta, dir.join(META_FILE))?;

    Ok(dir)
}

/// Load a workspace index from disk.
pub fn load_workspace(roots: &[RootInfo]) -> anyhow::Result<Option<WorkspaceIndex>> {
    let ws_hash = compute_workspace_hash(roots);
    let index_path = workspace_cache_dir(&ws_hash).join(INDEX_FILE);

    if !index_path.exists() {
        return Ok(None);
    }

    let data = fs::read(&index_path)?;
    let (mut index, _): (WorkspaceIndex, _) =
        bincode::serde::decode_from_slice(&data, bincode::config::standard())?;

    if index.version < CURRENT_INDEX_VERSION {
        return Ok(None);
    }

    // Rebuild call graph indexes after deserialization.
    // v2+ indices serialize name_to_idx and short_name_idx, so only the
    // petgraph DiGraph (which cannot impl serde) needs rebuilding.
    // v1 indices need the full rebuild including name_to_idx.
    if let Some(ref mut cg) = index.call_graph {
        if index.version >= 2 {
            cg.rebuild_digraph();
        } else {
            cg.rebuild_index();
        }
    }

    // v2+ indices serialize search_cache; only v1 needs rebuild.
    if index.version < 2 {
        index.rebuild_search_cache();
    }

    Ok(Some(index))
}
