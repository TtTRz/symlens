use crate::model::project::ProjectIndex;
use crate::search::bm25::SearchEngine;
use std::fs;
use std::path::{Path, PathBuf};

const INDEX_FILE: &str = "index.bin";
const META_FILE: &str = "meta.json";
const SEARCH_DIR: &str = "search";

/// Get the cache directory for a project.
pub fn cache_dir(root_hash: &str) -> PathBuf {
    let home = dirs_or_default();
    home.join(".symlens").join("indexes").join(root_hash)
}

/// Save a project index to disk, including tantivy search index.
pub fn save(index: &ProjectIndex) -> anyhow::Result<PathBuf> {
    let dir = cache_dir(&index.root_hash);
    fs::create_dir_all(&dir)?;

    // Save index as bincode
    let index_path = dir.join(INDEX_FILE);
    let encoded = bincode::serde::encode_to_vec(index, bincode::config::standard())?;
    fs::write(&index_path, encoded)?;

    // Build tantivy search index
    let search_dir = dir.join(SEARCH_DIR);
    if search_dir.exists() {
        fs::remove_dir_all(&search_dir)?;
    }
    let engine = SearchEngine::create(&search_dir)?;
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
    fs::write(dir.join(META_FILE), serde_json::to_string_pretty(&meta)?)?;

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

    // Rebuild call graph name_to_idx and digraph (skipped by serde)
    if let Some(ref mut cg) = index.call_graph {
        cg.rebuild_index();
    }

    // Rebuild pre-computed search cache (skipped by serde)
    index.rebuild_search_cache();

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
