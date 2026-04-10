use crate::index::{indexer, storage};
use notify::{Event, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct WatchArgs {
    pub path: Option<String>,
}

pub fn run(path: Option<String>) -> anyhow::Result<()> {
    let root = match path {
        Some(p) => PathBuf::from(p).canonicalize()?,
        None => {
            let cwd = std::env::current_dir()?;
            storage::find_project_root(&cwd).unwrap_or(cwd)
        }
    };

    eprintln!("👁  Watching {} for changes...", root.display());
    eprintln!("   Press Ctrl+C to stop.");

    // Initial index
    let result = indexer::index_project(&root, 100_000)?;
    storage::save(&result.index)?;
    eprintln!("   Indexed {} symbols in {}ms", result.index.symbols.len(), result.duration_ms);

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(&root, RecursiveMode::Recursive)?;

    let mut last_rebuild = Instant::now();
    let debounce = Duration::from_millis(500);

    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Ok(event)) => {
                // Check if any modified file is a source file
                let has_source_change = event.paths.iter().any(|p| {
                    matches!(
                        p.extension().and_then(|e| e.to_str()),
                        Some("rs") | Some("ts") | Some("tsx") | Some("py") | Some("swift") | Some("go")
                    )
                });

                if has_source_change && last_rebuild.elapsed() > debounce {
                    let start = Instant::now();
                    match indexer::index_project(&root, 100_000) {
                        Ok(result) => {
                            if let Err(e) = storage::save(&result.index) {
                                eprintln!("   ⚠ Save failed: {}", e);
                            } else {
                                let ms = start.elapsed().as_millis();
                                eprintln!(
                                    "   ↻ Re-indexed: {} symbols ({}ms)",
                                    result.index.symbols.len(),
                                    ms,
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!("   ⚠ Index failed: {}", e);
                        }
                    }
                    last_rebuild = Instant::now();
                }
            }
            Ok(Err(e)) => eprintln!("   ⚠ Watch error: {}", e),
            Err(mpsc::RecvTimeoutError::Timeout) => {} // Normal timeout, loop
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(())
}
