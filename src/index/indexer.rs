use crate::graph::call_graph::CallGraph;
use crate::model::project::ProjectIndex;
use crate::parser::registry::LanguageRegistry;
use crate::parser::traits::{CallEdge, ImportInfo};
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

pub struct IndexResult {
    pub index: ProjectIndex,
    pub duration_ms: u64,
    pub files_scanned: usize,
    pub files_parsed: usize,
    pub files_skipped: usize,
}

/// Index a project directory using tree-sitter.
/// If `prev_index` is provided, only re-parse files whose mtime has changed.
pub fn index_project(root: &Path, max_files: usize) -> anyhow::Result<IndexResult> {
    index_project_incremental(root, max_files, None)
}

/// Incremental index: reuse symbols from prev_index for unchanged files.
pub fn index_project_incremental(
    root: &Path,
    max_files: usize,
    prev_index: Option<&ProjectIndex>,
) -> anyhow::Result<IndexResult> {
    let start = Instant::now();
    let registry = LanguageRegistry::new();

    // Walk files, respecting .gitignore
    let files: Vec<PathBuf> = WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .filter(|entry| registry.is_supported(entry.path()))
        .take(max_files)
        .map(|entry| entry.into_path())
        .collect();

    let files_scanned = files.len();

    // Parse files in parallel
    let index = Mutex::new(ProjectIndex::new(root.to_path_buf()));
    let files_parsed = Mutex::new(0usize);
    let files_skipped = Mutex::new(0usize);
    let all_call_edges: Mutex<Vec<CallEdge>> = Mutex::new(Vec::new());
    let all_imports: Mutex<Vec<(PathBuf, ImportInfo)>> = Mutex::new(Vec::new());

    files.par_iter().for_each(|file_path| {
        let rel_path = file_path.strip_prefix(root).unwrap_or(file_path);

        // Incremental: check if file is unchanged
        let current_mtime = std::fs::metadata(file_path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if let Some(prev) = prev_index {
            // Fast path: mtime unchanged → skip entirely
            if let Some(&prev_mtime) = prev.file_mtimes.get(rel_path)
                && prev_mtime == current_mtime
            {
                // File unchanged — copy symbols from previous index
                let mut idx = index.lock().expect("index lock poisoned");
                if let Some(sym_ids) = prev.file_symbols.get(rel_path) {
                    for sym_id in sym_ids {
                        if let Some(sym) = prev.symbols.get(sym_id) {
                            idx.insert(sym.clone());
                        }
                    }
                }
                idx.file_mtimes.insert(rel_path.to_path_buf(), prev_mtime);
                // Carry forward content hash
                if let Some(hash) = prev.file_hashes.get(rel_path) {
                    idx.file_hashes.insert(rel_path.to_path_buf(), hash.clone());
                }
                *files_skipped.lock().expect("counter lock poisoned") += 1;
                return;
            }

            // Slow path: mtime changed, check content hash to catch git checkout/rebase
            if let Ok(ref source) = std::fs::read(file_path) {
                let hash = blake3::hash(source).to_hex()[..16].to_string();
                if let Some(prev_hash) = prev.file_hashes.get(rel_path)
                    && hash == *prev_hash
                {
                    // Content unchanged despite mtime change — skip re-parsing
                    let mut idx = index.lock().expect("index lock poisoned");
                    if let Some(sym_ids) = prev.file_symbols.get(rel_path) {
                        for sym_id in sym_ids {
                            if let Some(sym) = prev.symbols.get(sym_id) {
                                idx.insert(sym.clone());
                            }
                        }
                    }
                    idx.file_mtimes
                        .insert(rel_path.to_path_buf(), current_mtime);
                    idx.file_hashes.insert(rel_path.to_path_buf(), hash);
                    *files_skipped.lock().expect("counter lock poisoned") += 1;
                    return;
                }
            }
        }

        if let Some(parser) = registry.parser_for(file_path)
            && let Ok(source) = std::fs::read(file_path)
        {
            if let Ok(symbols) = parser.extract_symbols(&source, rel_path) {
                let mut idx = index.lock().expect("index lock poisoned");
                for symbol in symbols {
                    idx.insert(symbol);
                }
                idx.file_mtimes
                    .insert(rel_path.to_path_buf(), current_mtime);
                // Store content hash for future incremental checks
                let hash = blake3::hash(&source).to_hex()[..16].to_string();
                idx.file_hashes.insert(rel_path.to_path_buf(), hash);
                *files_parsed.lock().expect("counter lock poisoned") += 1;
            }

            if let Ok(edges) = parser.extract_calls(&source, rel_path)
                && !edges.is_empty()
            {
                all_call_edges
                    .lock()
                    .expect("edges lock poisoned")
                    .extend(edges);
            }

            if let Ok(imps) = parser.extract_imports(&source, rel_path)
                && !imps.is_empty()
            {
                let mut all = all_imports.lock().expect("imports lock poisoned");
                for imp in imps {
                    all.push((rel_path.to_path_buf(), imp));
                }
            }
        }
    });

    let duration_ms = start.elapsed().as_millis() as u64;
    let files_parsed = *files_parsed.lock().expect("counter lock poisoned");
    let files_skipped = *files_skipped.lock().expect("counter lock poisoned");
    let mut index = index.into_inner().expect("index lock poisoned");

    // Build call graph
    let call_edges = all_call_edges.into_inner().expect("edges lock poisoned");
    if !call_edges.is_empty() {
        index.call_graph = Some(CallGraph::build(&call_edges));
    }

    // Build import name → files mapping
    let import_data = all_imports.into_inner().expect("imports lock poisoned");
    for (file, imp) in &import_data {
        for name in &imp.names {
            index
                .import_names
                .entry(name.clone())
                .or_default()
                .push(file.clone());
        }
    }

    Ok(IndexResult {
        index,
        duration_ms,
        files_scanned,
        files_parsed,
        files_skipped,
    })
}
