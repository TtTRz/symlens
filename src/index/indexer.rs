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
}

/// Index a project directory using tree-sitter.
pub fn index_project(root: &Path, max_files: usize) -> anyhow::Result<IndexResult> {
    let start = Instant::now();
    let registry = LanguageRegistry::new();

    // Walk files, respecting .gitignore
    let files: Vec<PathBuf> = WalkBuilder::new(root)
        .hidden(true) // skip dotfiles
        .git_ignore(true) // respect .gitignore
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

    // Parse all files in parallel
    let index = Mutex::new(ProjectIndex::new(root.to_path_buf()));
    let files_parsed = Mutex::new(0usize);
    let all_call_edges: Mutex<Vec<CallEdge>> = Mutex::new(Vec::new());
    let all_imports: Mutex<Vec<(PathBuf, ImportInfo)>> = Mutex::new(Vec::new());

    files.par_iter().for_each(|file_path| {
        let rel_path = file_path.strip_prefix(root).unwrap_or(file_path);

        if let Some(parser) = registry.parser_for(file_path) {
            if let Ok(source) = std::fs::read(file_path) {
                // Extract symbols
                if let Ok(symbols) = parser.extract_symbols(&source, rel_path) {
                    let mtime = std::fs::metadata(file_path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);

                    let mut idx = index.lock().unwrap();
                    for symbol in symbols {
                        idx.insert(symbol);
                    }
                    idx.file_mtimes.insert(rel_path.to_path_buf(), mtime);
                    *files_parsed.lock().unwrap() += 1;
                }

                // Extract call edges
                if let Ok(edges) = parser.extract_calls(&source, rel_path) {
                    if !edges.is_empty() {
                        all_call_edges.lock().unwrap().extend(edges);
                    }
                }

                // Extract imports
                if let Ok(imps) = parser.extract_imports(&source, rel_path) {
                    if !imps.is_empty() {
                        let mut all = all_imports.lock().unwrap();
                        for imp in imps {
                            all.push((rel_path.to_path_buf(), imp));
                        }
                    }
                }
            }
        }
    });

    let duration_ms = start.elapsed().as_millis() as u64;
    let files_parsed = *files_parsed.lock().unwrap();
    let mut index = index.into_inner().unwrap();

    // Build call graph
    let call_edges = all_call_edges.into_inner().unwrap();
    if !call_edges.is_empty() {
        index.call_graph = Some(CallGraph::build(&call_edges));
    }

    // Build import name → files mapping (for refs v3)
    let import_data = all_imports.into_inner().unwrap();
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
    })
}
