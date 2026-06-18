use crate::commands::IndexProvider;
use crate::daemon::rpc;
use crate::daemon::{SharedIndex, socket_path};
use crate::index::{indexer, storage};
use crate::model::project::ProjectIndex;
use crate::parser::traits::is_source_file;
use notify::{Event, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::io::{BufRead, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, mpsc};
use std::time::{Duration, Instant};

pub fn serve_daemon(root_override: Option<&str>, workspace_flag: bool, no_ignore: bool) -> anyhow::Result<()> {
    let root_owned = root_override.map(String::from);

    // Load initial index
    let provider = IndexProvider::load(root_override, workspace_flag)?;
    let is_workspace = provider.is_workspace();
    let hash = provider.socket_hash();

    // Extract root info before moving provider into SharedIndex
    let roots_info: Vec<(String, PathBuf, String, String)> = provider
        .roots()
        .into_iter()
        .map(|(id, path, hash, label)| {
            (
                id.to_string(),
                path.to_path_buf(),
                hash.to_string(),
                label.to_string(),
            )
        })
        .collect();

    let root_paths: Vec<PathBuf> = if !is_workspace {
        vec![roots_info[0].1.clone()]
    } else {
        roots_info
            .iter()
            .map(|(_, path, _, _)| path.clone())
            .collect()
    };

    let shared: SharedIndex = Arc::new(RwLock::new(provider));
    let shutdown = Arc::new(AtomicBool::new(false));

    // Socket setup
    let sock_path = socket_path(&hash, is_workspace);
    if let Some(parent) = sock_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if sock_path.exists() {
        std::fs::remove_file(&sock_path)?;
    }

    let listener = UnixListener::bind(&sock_path)?;
    listener.set_nonblocking(true)?;

    eprintln!("symlens daemon listening on {}", sock_path.display());
    eprintln!("   Roots:");
    for (_, path, _, label) in &roots_info {
        eprintln!("     - {} ({})", path.display(), label);
    }
    eprintln!("   Press Ctrl+C to stop.");

    // Spawn watcher thread
    let watcher_shared = shared.clone();
    let shutdown_watcher = shutdown.clone();
    std::thread::spawn(move || {
        run_watcher(
            root_paths,
            watcher_shared,
            is_workspace,
            root_owned.as_deref(),
            &shutdown_watcher,
            no_ignore,
        );
    });

    // Accept loop
    loop {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }

        match listener.accept() {
            Ok((stream, _)) => {
                let shared = shared.clone();
                std::thread::spawn(move || {
                    handle_connection(stream, &shared);
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => {
                eprintln!("Accept error: {}", e);
                break;
            }
        }
    }

    let _ = std::fs::remove_file(&sock_path);
    eprintln!("Daemon stopped, socket cleaned up.");
    Ok(())
}

/// Handle a client connection. Supports multiple requests on the same connection.
fn handle_connection(stream: UnixStream, index: &SharedIndex) {
    // Reset to blocking mode — the accepted stream inherits nonblocking from the listener.
    let _ = stream.set_nonblocking(false);

    let mut reader = std::io::BufReader::new(stream);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            _ => {}
        }

        let response = rpc::handle_request(line.trim_end(), index);

        let stream = reader.get_mut();
        if stream.write_all(response.as_bytes()).is_err() || stream.write_all(b"\n").is_err() {
            return;
        }
    }
}

fn run_watcher(
    root_paths: Vec<PathBuf>,
    shared: SharedIndex,
    is_workspace: bool,
    root_override: Option<&str>,
    shutdown: &AtomicBool,
    no_ignore: bool,
) {
    let walk_opts = crate::index::indexer::WalkOptions { respect_gitignore: !no_ignore };
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = match notify::recommended_watcher(tx) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Watcher error: {}", e);
            return;
        }
    };

    for root in &root_paths {
        if let Err(e) = watcher.watch(root, RecursiveMode::Recursive) {
            eprintln!("Watch error for {}: {}", root.display(), e);
        }
    }

    let mut prev_index: Option<ProjectIndex> = None;
    let mut pending_files: HashSet<PathBuf> = HashSet::new();
    let mut last_event = Instant::now();
    let min_debounce = Duration::from_millis(500);

    loop {
        if shutdown.load(Ordering::SeqCst) {
            return;
        }

        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => {
                for p in &event.paths {
                    if is_source_file(p) {
                        pending_files.insert(p.clone());
                        last_event = Instant::now();
                    }
                }
            }
            Ok(Err(e)) => eprintln!("Watch error: {}", e),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }

        if !pending_files.is_empty() && last_event.elapsed() > min_debounce {
            for root in &root_paths {
                let has_changes = pending_files.iter().any(|f| f.starts_with(root));
                if !has_changes {
                    continue;
                }

                let start = Instant::now();
                match indexer::index_project_incremental(root, 100_000, prev_index.as_ref(), &walk_opts) {
                    Ok(result) => {
                        let sym_count = result.index.symbols.len();
                        // Save to disk first
                        if let Err(e) = storage::save(&result.index) {
                            eprintln!("Save failed for {}: {}", root.display(), e);
                        } else {
                            eprintln!(
                                "Re-indexed {}: {} symbols ({}ms)",
                                root.display(),
                                sym_count,
                                start.elapsed().as_millis(),
                            );
                        }

                        // Swap the in-memory shared index
                        if is_workspace {
                            if let Ok(mut guard) = shared.write()
                                && let Ok(new_provider) = IndexProvider::load(root_override, true)
                            {
                                *guard = new_provider;
                            }
                        } else {
                            let new_provider =
                                IndexProvider::from_single(root.clone(), result.index);
                            if let Ok(mut guard) = shared.write() {
                                *guard = new_provider;
                                // Extract prev_index from the new provider to avoid disk reload
                                prev_index = match &*guard {
                                    IndexProvider::Single { index, .. } => Some(index.clone()),
                                    IndexProvider::Workspace { .. } => None,
                                };
                            }
                        }

                        // For workspace mode, prev_index is not used (each root is tracked
                        // separately by the workspace reload). For single-root, we already
                        // set prev_index above via clone from the write lock guard.
                    }
                    Err(e) => eprintln!("Index failed for {}: {}", root.display(), e),
                }
            }

            pending_files.clear();
        }
    }
}
