use crate::graph::call_graph::CallGraph;
use crate::model::project::{ProjectIndex, RootInfo};
use crate::model::symbol::Symbol;
use crate::model::workspace::WorkspaceIndex;
use crate::parser::registry::LanguageRegistry;
use crate::parser::traits::{CallEdge, ImportInfo};
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct IndexResult {
    pub index: ProjectIndex,
    pub duration_ms: u64,
    pub files_scanned: usize,
    pub files_parsed: usize,
    pub files_skipped: usize,
}

/// Per-file parsing result collected in parallel, merged sequentially.
#[derive(Default)]
struct FileResult {
    symbols: Vec<Symbol>,
    file_mtime: Option<(PathBuf, u64)>,
    file_hash: Option<(PathBuf, String)>,
    call_edges: Vec<CallEdge>,
    file_call_edges: Option<(PathBuf, Vec<CallEdge>)>,
    imports: Vec<(PathBuf, ImportInfo)>,
    file_imports: Option<(PathBuf, Vec<ImportInfo>)>,
    parsed: bool,
    skipped: bool,
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

    // Parse files in parallel — lock-free collection via map-reduce
    let results: Vec<FileResult> = files
        .par_iter()
        .map(|file_path| {
            let rel_path = file_path.strip_prefix(root).unwrap_or(file_path);
            let mut result = FileResult::default();

            // Incremental: check if file is unchanged
            let current_mtime = std::fs::metadata(file_path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            if let Some(prev) = prev_index {
                // Fast path: mtime unchanged → reuse previous results
                if let Some(&prev_mtime) = prev.file_mtimes.get(rel_path)
                    && prev_mtime == current_mtime
                {
                    copy_prev_data(prev, rel_path, prev_mtime, &mut result);
                    result.skipped = true;
                    return result;
                }

                // Slow path: mtime changed, check content hash to catch git checkout/rebase
                if let Ok(ref source) = std::fs::read(file_path) {
                    let hash = blake3::hash(source).to_hex()[..16].to_string();
                    if let Some(prev_hash) = prev.file_hashes.get(rel_path)
                        && hash == *prev_hash
                    {
                        // Content unchanged despite mtime change
                        copy_prev_data(prev, rel_path, current_mtime, &mut result);
                        result.file_hash = Some((rel_path.to_path_buf(), hash));
                        result.skipped = true;
                        return result;
                    }
                }
            }

            // Full parse path: single parse, extract all data at once
            if let Some(parser) = registry.parser_for(file_path)
                && let Ok(source) = std::fs::read(file_path)
            {
                match parser.extract_all(&source, rel_path) {
                    Ok(output) => {
                        result.symbols = output.symbols;
                        result.file_mtime = Some((rel_path.to_path_buf(), current_mtime));
                        let hash = blake3::hash(&source).to_hex()[..16].to_string();
                        result.file_hash = Some((rel_path.to_path_buf(), hash));
                        result.parsed = true;

                        if !output.call_edges.is_empty() {
                            result.file_call_edges =
                                Some((rel_path.to_path_buf(), output.call_edges.clone()));
                            result.call_edges = output.call_edges;
                        }

                        if !output.imports.is_empty() {
                            result.file_imports =
                                Some((rel_path.to_path_buf(), output.imports.clone()));
                            for imp in output.imports {
                                result.imports.push((rel_path.to_path_buf(), imp));
                            }
                        }
                    }
                    Err(_) => {
                        // Fallback: try extract_symbols only
                        if let Ok(symbols) = parser.extract_symbols(&source, rel_path) {
                            result.symbols = symbols;
                            result.file_mtime = Some((rel_path.to_path_buf(), current_mtime));
                            let hash = blake3::hash(&source).to_hex()[..16].to_string();
                            result.file_hash = Some((rel_path.to_path_buf(), hash));
                            result.parsed = true;
                        }
                    }
                }
            }

            result
        })
        .collect();

    // Sequential merge — single-threaded, no locks needed
    let mut index = ProjectIndex::new(root.to_path_buf());
    let mut all_call_edges: Vec<CallEdge> = Vec::new();
    let mut files_parsed: usize = 0;
    let mut files_skipped: usize = 0;

    for r in results {
        for sym in r.symbols {
            index.insert(sym);
        }
        if let Some((path, mtime)) = r.file_mtime {
            index.file_mtimes.insert(path, mtime);
        }
        if let Some((path, hash)) = r.file_hash {
            index.file_hashes.insert(path, hash);
        }
        if let Some((path, edges)) = r.file_call_edges {
            index.file_call_edges.insert(path, edges);
        }
        if let Some((path, imps)) = r.file_imports {
            index.file_imports.insert(path, imps);
        }
        all_call_edges.extend(r.call_edges);
        for (file, imp) in &r.imports {
            for name in &imp.names {
                index
                    .import_names
                    .entry(name.clone())
                    .or_default()
                    .push(file.clone());
            }
        }
        if r.parsed {
            files_parsed += 1;
        }
        if r.skipped {
            files_skipped += 1;
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    // Build call graph from all collected edges
    if !all_call_edges.is_empty() {
        index.call_graph = Some(CallGraph::build(&all_call_edges));
    }

    Ok(IndexResult {
        index,
        duration_ms,
        files_scanned,
        files_parsed,
        files_skipped,
    })
}

/// Copy symbols, edges, and imports from a previous index for an unchanged file.
fn copy_prev_data(prev: &ProjectIndex, rel_path: &Path, mtime: u64, result: &mut FileResult) {
    if let Some(sym_ids) = prev.file_symbols.get(rel_path) {
        for sym_id in sym_ids {
            if let Some(sym) = prev.symbols.get(sym_id) {
                result.symbols.push(sym.clone());
            }
        }
    }
    result.file_mtime = Some((rel_path.to_path_buf(), mtime));
    if let Some(hash) = prev.file_hashes.get(rel_path) {
        result.file_hash = Some((rel_path.to_path_buf(), hash.clone()));
    }
    if let Some(prev_edges) = prev.file_call_edges.get(rel_path) {
        result.call_edges.extend(prev_edges.clone());
        result.file_call_edges = Some((rel_path.to_path_buf(), prev_edges.clone()));
    }
    if let Some(prev_imps) = prev.file_imports.get(rel_path) {
        for imp in prev_imps {
            result.imports.push((rel_path.to_path_buf(), imp.clone()));
        }
        result.file_imports = Some((rel_path.to_path_buf(), prev_imps.clone()));
    }
}

// ---------------------------------------------------------------------------
// Workspace indexing
// ---------------------------------------------------------------------------

pub struct WorkspaceIndexResult {
    pub index: WorkspaceIndex,
    pub duration_ms: u64,
    pub files_scanned: usize,
    pub files_parsed: usize,
    pub files_skipped: usize,
}

/// Index a workspace with multiple project roots.
/// Each root is indexed independently (with per-root incremental support),
/// then merged into a single WorkspaceIndex with `[root_id]` prefixes.
pub fn index_workspace(
    roots: &[RootInfo],
    max_files_per_root: usize,
    _prev_workspace: Option<&WorkspaceIndex>,
) -> anyhow::Result<WorkspaceIndexResult> {
    let start = Instant::now();
    let mut ws = WorkspaceIndex::new(roots);

    let mut total_scanned = 0usize;
    let mut total_parsed = 0usize;
    let mut total_skipped = 0usize;

    for root_info in roots {
        // Per-root incremental: try loading per-root cache from disk.
        // Full workspace prev is not used for per-root incremental — each root
        // has its own cache file on disk.
        let prev_root_index = crate::index::storage::load(&root_info.path).ok().flatten();

        let result = index_project_incremental(
            &root_info.path,
            max_files_per_root,
            prev_root_index.as_ref(),
        )?;

        // Save per-root cache (enables single-root load + incremental for next workspace run)
        if let Err(e) = crate::index::storage::save(&result.index) {
            eprintln!(
                "warning: failed to save per-root cache for {}: {e}",
                root_info.path.display()
            );
        }

        // Merge into workspace
        ws.insert_from_project(root_info, &result.index);

        total_scanned += result.files_scanned;
        total_parsed += result.files_parsed;
        total_skipped += result.files_skipped;
    }

    // Build unified call graph from all merged call edges
    ws.build_call_graph();
    ws.indexed_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(WorkspaceIndexResult {
        index: ws,
        duration_ms,
        files_scanned: total_scanned,
        files_parsed: total_parsed,
        files_skipped: total_skipped,
    })
}
