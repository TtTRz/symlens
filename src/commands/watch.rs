use crate::index::{indexer, storage};
use crate::model::project::ProjectIndex;
use crate::parser::traits::is_source_file;
use notify::{Event, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub fn run(path: Option<&str>, workspace_flag: bool, no_ignore: bool) -> anyhow::Result<()> {
    let provider = crate::commands::IndexProvider::load(path, workspace_flag)?;

    let roots = provider.roots();
    let walk_opts = indexer::WalkOptions {
        respect_gitignore: !no_ignore,
    };

    if roots.len() == 1 {
        // Single root mode — same as before
        let root = roots[0].1.to_path_buf();
        watch_single_root(&root, &walk_opts)?;
    } else {
        // Workspace mode — watch each root independently
        eprintln!("👁  Watching {} roots for changes...", roots.len());
        for (_, root_path, _, _) in &roots {
            eprintln!("   - {}", root_path.display());
        }
        eprintln!("   Press Ctrl+C to stop.");

        // For workspace mode, watch all roots in sequence.
        // This is a simplified implementation: we watch roots one by one
        // and rebuild the full workspace index on any change.
        watch_workspace(&roots, &walk_opts)?;
    }

    Ok(())
}

fn watch_single_root(
    root: &std::path::Path,
    walk_opts: &indexer::WalkOptions,
) -> anyhow::Result<()> {
    eprintln!("👁  Watching {} for changes...", root.display());
    eprintln!("   Press Ctrl+C to stop.");

    // Initial index
    let result = indexer::index_project_incremental(root, 100_000, None, walk_opts)?;
    storage::save(&result.index)?;
    eprintln!(
        "   Indexed {} symbols in {}ms",
        result.index.symbols.len(),
        result.duration_ms
    );

    // Store previous index for incremental reuse
    let mut prev_index: Option<ProjectIndex> = Some(result.index);

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(root, RecursiveMode::Recursive)?;

    let mut pending_files: HashSet<PathBuf> = HashSet::new();
    let mut last_event = Instant::now();
    let min_debounce = Duration::from_millis(500);

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => {
                for p in &event.paths {
                    if is_source_file(p) {
                        pending_files.insert(p.clone());
                        last_event = Instant::now();
                    }
                }
            }
            Ok(Err(e)) => eprintln!("   ⚠ Watch error: {}", e),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        // Adaptive debounce: wait at least min_debounce since last event
        if !pending_files.is_empty() && last_event.elapsed() > min_debounce {
            let start = Instant::now();
            match indexer::index_project_incremental(root, 100_000, prev_index.as_ref(), walk_opts)
            {
                Ok(result) => {
                    if let Err(e) = storage::save(&result.index) {
                        eprintln!("   ⚠ Save failed: {}", e);
                    } else {
                        eprintln!(
                            "   ↻ Re-indexed: {} symbols ({}ms, {} parsed, {} skipped)",
                            result.index.symbols.len(),
                            start.elapsed().as_millis(),
                            result.files_parsed,
                            result.files_skipped,
                        );
                    }
                    prev_index = Some(result.index);
                }
                Err(e) => eprintln!("   ⚠ Index failed: {}", e),
            }
            pending_files.clear();
        }
    }

    Ok(())
}

fn watch_workspace(
    roots: &[(&str, &std::path::Path, &str, &str)],
    walk_opts: &indexer::WalkOptions,
) -> anyhow::Result<()> {
    // Initial index for each root
    let mut prev_indices: Vec<(String, PathBuf, Option<ProjectIndex>)> = Vec::new();
    for (_root_id, root_path, _root_hash, root_label) in roots {
        match indexer::index_project_incremental(root_path, 100_000, None, walk_opts) {
            Ok(result) => {
                eprintln!(
                    "   [{}] Indexed {} symbols in {}ms",
                    root_label,
                    result.index.symbols.len(),
                    result.duration_ms
                );
                if let Err(e) = storage::save(&result.index) {
                    eprintln!("   [{}] ⚠ Save failed: {}", root_label, e);
                }
                prev_indices.push((
                    root_label.to_string(),
                    root_path.to_path_buf(),
                    Some(result.index),
                ));
            }
            Err(e) => {
                eprintln!("   [{}] ⚠ Index failed: {}", root_label, e);
                prev_indices.push((root_label.to_string(), root_path.to_path_buf(), None));
            }
        }
    }

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx)?;

    for (_, root_path, _, _) in roots {
        watcher.watch(root_path, RecursiveMode::Recursive)?;
    }

    let mut pending_files: HashSet<PathBuf> = HashSet::new();
    let mut last_event = Instant::now();
    let min_debounce = Duration::from_millis(500);

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => {
                for p in &event.paths {
                    if is_source_file(p) {
                        pending_files.insert(p.clone());
                        last_event = Instant::now();
                    }
                }
            }
            Ok(Err(e)) => eprintln!("   ⚠ Watch error: {}", e),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        // Re-index only the root(s) that contain changed files
        if !pending_files.is_empty() && last_event.elapsed() > min_debounce {
            let start = Instant::now();

            // Determine which roots need re-indexing
            for (root_id, root_path, prev_index) in &mut prev_indices {
                let has_changes = pending_files.iter().any(|f| f.starts_with(&*root_path));
                if !has_changes {
                    continue;
                }

                match indexer::index_project_incremental(
                    root_path,
                    100_000,
                    prev_index.as_ref(),
                    walk_opts,
                ) {
                    Ok(result) => {
                        if let Err(e) = storage::save(&result.index) {
                            eprintln!("   [{}] ⚠ Save failed: {}", root_id, e);
                        } else {
                            eprintln!(
                                "   [{}] ↻ Re-indexed: {} symbols ({}ms, {} parsed, {} skipped)",
                                root_id,
                                result.index.symbols.len(),
                                start.elapsed().as_millis(),
                                result.files_parsed,
                                result.files_skipped,
                            );
                        }
                        *prev_index = Some(result.index);
                    }
                    Err(e) => eprintln!("   [{}] ⚠ Index failed: {}", root_id, e),
                }
            }

            pending_files.clear();
        }
    }

    Ok(())
}
