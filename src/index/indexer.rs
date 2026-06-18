use crate::graph::call_graph::CallGraph;
use crate::model::project::{ProjectIndex, RootInfo};
use crate::model::symbol::Symbol;
use crate::model::workspace::WorkspaceIndex;
use crate::parser::traits::{CallEdge, IdentifierRef, ImportInfo};
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
    pub files_truncated: usize,
    pub files_failed: usize,
    pub failed_paths: Vec<PathBuf>,
}

/// Per-file parsing result collected in parallel, merged sequentially.
#[derive(Default)]
struct FileResult {
    rel_path: Option<PathBuf>,
    symbols: Vec<Symbol>,
    file_mtime: Option<(PathBuf, u64)>,
    file_hash: Option<(PathBuf, String)>,
    file_call_edges: Option<(PathBuf, Vec<CallEdge>)>,
    file_imports: Option<(PathBuf, Vec<ImportInfo>)>,
    file_identifiers: Option<(PathBuf, Vec<crate::parser::traits::IdentifierRef>)>,
    parsed: bool,
    skipped: bool,
    failed: bool,
    degraded: bool,
}

/// Controls `WalkBuilder` behavior for index operations.
#[derive(Clone, Debug)]
pub struct WalkOptions {
    /// When `true` (default), respect `.gitignore`, `.git/info/exclude`, and global gitignore.
    pub respect_gitignore: bool,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self { respect_gitignore: true }
    }
}

/// Index a project directory using tree-sitter.
/// If `prev_index` is provided, only re-parse files whose mtime has changed.
pub fn index_project(root: &Path, max_files: usize) -> anyhow::Result<IndexResult> {
    index_project_incremental(root, max_files, None, &WalkOptions::default())
}

/// Incremental index: reuse symbols from prev_index for unchanged files.
pub fn index_project_incremental(
    root: &Path,
    max_files: usize,
    prev_index: Option<&ProjectIndex>,
    walk_opts: &WalkOptions,
) -> anyhow::Result<IndexResult> {
    // Load prev identifiers separately (stored in idents.bin, not in main index)
    let prev_idents: Option<
        std::sync::Arc<std::collections::HashMap<PathBuf, Vec<IdentifierRef>>>,
    > = prev_index
        .and_then(|_| crate::index::storage::load_identifiers(root).ok().flatten())
        .map(|(fi, _)| std::sync::Arc::new(fi));
    let start = Instant::now();
    let registry = &*crate::parser::registry::GLOBAL_REGISTRY;

    // Walk files, respecting .gitignore
    let all_files: Vec<PathBuf> = WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(walk_opts.respect_gitignore)
        .git_global(walk_opts.respect_gitignore)
        .git_exclude(walk_opts.respect_gitignore)
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .filter(|entry| registry.is_supported(entry.path()))
        .map(|entry| entry.into_path())
        .collect();

    let files_truncated = all_files.len().saturating_sub(max_files);
    let files: Vec<PathBuf> = all_files.into_iter().take(max_files).collect();

    let files_scanned = files.len();

    // Parse files in parallel — lock-free collection via map-reduce
    let results: Vec<FileResult> = files
        .par_iter()
        .map(|file_path| {
            let rel_path = file_path.strip_prefix(root).unwrap_or(file_path);
            let mut result = FileResult::default();
            result.rel_path = Some(rel_path.to_path_buf());

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
                    copy_prev_data(prev, &prev_idents, rel_path, prev_mtime, &mut result);
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
                        copy_prev_data(prev, &prev_idents, rel_path, current_mtime, &mut result);
                        result.file_hash = Some((rel_path.to_path_buf(), hash));
                        result.skipped = true;
                        return result;
                    }
                }
            }

            // Full parse path: single parse, extract all data at once
            if let Some(parser) = registry.parser_for(file_path) {
                let source = match std::fs::read(file_path) {
                    Ok(s) => s,
                    Err(_) => {
                        result.failed = true;
                        return result;
                    }
                };
                match parser.extract_all(&source, rel_path) {
                    Ok(output) => {
                        result.symbols = output.symbols;
                        result.file_mtime = Some((rel_path.to_path_buf(), current_mtime));
                        let hash = blake3::hash(&source).to_hex()[..16].to_string();
                        result.file_hash = Some((rel_path.to_path_buf(), hash));
                        result.parsed = true;

                        if !output.call_edges.is_empty() {
                            result.file_call_edges =
                                Some((rel_path.to_path_buf(), output.call_edges));
                        }
                        if !output.imports.is_empty() {
                            result.file_imports = Some((rel_path.to_path_buf(), output.imports));
                        }
                        if !output.identifiers.is_empty() {
                            result.file_identifiers =
                                Some((rel_path.to_path_buf(), output.identifiers));
                        }
                    }
                    Err(_) => match parser.extract_symbols(&source, rel_path) {
                        Ok(symbols) => {
                            result.symbols = symbols;
                            result.file_mtime = Some((rel_path.to_path_buf(), current_mtime));
                            let hash = blake3::hash(&source).to_hex()[..16].to_string();
                            result.file_hash = Some((rel_path.to_path_buf(), hash));
                            result.parsed = true;
                            result.degraded = true;
                        }
                        Err(_) => {
                            result.failed = true;
                        }
                    },
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
    let mut files_failed: usize = 0;
    let mut failed_paths: Vec<PathBuf> = Vec::new();

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
            all_call_edges.extend(edges.iter().cloned());
            index.file_call_edges.insert(path, edges);
        }
        if let Some((path, imps)) = r.file_imports {
            for imp in &imps {
                for name in &imp.names {
                    index
                        .import_names
                        .entry(name.clone())
                        .or_default()
                        .push(path.clone());
                }
            }
            index.file_imports.insert(path, imps);
        }
        if let Some((path, idents)) = r.file_identifiers {
            let mut seen_names = std::collections::HashSet::new();
            for ident in &idents {
                if seen_names.insert(ident.name.as_str()) {
                    index
                        .identifier_index
                        .entry(ident.name.clone())
                        .or_default()
                        .push(path.clone());
                }
            }
            index.file_identifiers.insert(path, idents);
        }
        if r.parsed {
            files_parsed += 1;
        }
        if r.skipped {
            files_skipped += 1;
        }
        if r.failed {
            files_failed += 1;
            if failed_paths.len() < 50 {
                if let Some(p) = r.rel_path.as_ref() {
                    failed_paths.push(p.clone());
                }
            }
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    // Build call graph from all collected edges
    if !all_call_edges.is_empty() {
        index.call_graph = Some(CallGraph::build(&all_call_edges));
    }

    index.files_truncated = files_truncated;
    index.files_failed = files_failed;
    index.failed_paths = failed_paths.clone();

    Ok(IndexResult {
        index,
        duration_ms,
        files_scanned,
        files_parsed,
        files_skipped,
        files_truncated,
        files_failed,
        failed_paths,
    })
}

/// Copy symbols, edges, and imports from a previous index for an unchanged file.
fn copy_prev_data(
    prev: &ProjectIndex,
    prev_idents: &Option<std::sync::Arc<std::collections::HashMap<PathBuf, Vec<IdentifierRef>>>>,
    rel_path: &Path,
    mtime: u64,
    result: &mut FileResult,
) {
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
        result.file_call_edges = Some((rel_path.to_path_buf(), prev_edges.clone()));
    }
    if let Some(prev_imps) = prev.file_imports.get(rel_path) {
        result.file_imports = Some((rel_path.to_path_buf(), prev_imps.clone()));
    }
    if let Some(idents_map) = prev_idents
        && let Some(prev_idents) = idents_map.get(rel_path)
    {
        result.file_identifiers = Some((rel_path.to_path_buf(), prev_idents.clone()));
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
    pub files_truncated: usize,
    pub files_failed: usize,
    pub failed_paths: Vec<PathBuf>,
}

/// Index a workspace with multiple project roots.
/// Each root is indexed independently (with per-root incremental support),
/// then merged into a single WorkspaceIndex with `[root_id]` prefixes.
pub fn index_workspace(
    roots: &[RootInfo],
    max_files_per_root: usize,
    _prev_workspace: Option<&WorkspaceIndex>,
    walk_opts: &WalkOptions,
) -> anyhow::Result<WorkspaceIndexResult> {
    let start = Instant::now();
    let mut ws = WorkspaceIndex::new(roots);

    let mut total_scanned = 0usize;
    let mut total_parsed = 0usize;
    let mut total_skipped = 0usize;
    let mut total_truncated = 0usize;
    let mut total_failed = 0usize;
    let mut all_failed_paths: Vec<PathBuf> = Vec::new();

    for root_info in roots {
        // Per-root incremental: try loading per-root cache from disk.
        // Full workspace prev is not used for per-root incremental — each root
        // has its own cache file on disk.
        let prev_root_index = crate::index::storage::load(&root_info.path).ok().flatten();

        let result = index_project_incremental(
            &root_info.path,
            max_files_per_root,
            prev_root_index.as_ref(),
            walk_opts,
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
        total_truncated += result.files_truncated;
        total_failed += result.files_failed;
        if all_failed_paths.len() < 50 {
            let remaining = 50 - all_failed_paths.len();
            all_failed_paths.extend(result.failed_paths.into_iter().take(remaining));
        }
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
        files_truncated: total_truncated,
        files_failed: total_failed,
        failed_paths: all_failed_paths,
    })
}
