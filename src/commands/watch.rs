use crate::index::{indexer, storage};
use crate::model::project::ProjectIndex;
use notify::{Event, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub fn run(path: Option<&str>) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(path)?;

    eprintln!("👁  Watching {} for changes...", root.display());
    eprintln!("   Press Ctrl+C to stop.");

    // Initial index
    let result = indexer::index_project(&root, 100_000)?;
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
    watcher.watch(&root, RecursiveMode::Recursive)?;

    let mut pending_files: HashSet<PathBuf> = HashSet::new();
    let mut last_event = Instant::now();
    let min_debounce = Duration::from_millis(500);

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => {
                for p in &event.paths {
                    if matches!(
                        p.extension().and_then(|e| e.to_str()),
                        Some("rs")
                            | Some("ts")
                            | Some("tsx")
                            | Some("py")
                            | Some("swift")
                            | Some("go")
                            | Some("dart")
                            | Some("c")
                            | Some("h")
                            | Some("cpp")
                            | Some("cc")
                            | Some("cxx")
                            | Some("hpp")
                            | Some("hh")
                            | Some("kt")
                            | Some("kts")
                    ) {
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
            match indexer::index_project_incremental(&root, 100_000, prev_index.as_ref()) {
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
