/// Integration tests for the daemon module.
/// Tests cover SharedIndex concurrency, socket path resolution, RPC protocol,
/// and end-to-end daemon lifecycle with real Unix sockets.
#[cfg(test)]
mod tests {
    use parking_lot::RwLock;
    use std::path::PathBuf;
    use std::sync::Arc;

    /// Verify that a minimal IndexProvider can be created and wrapped in SharedIndex.
    #[test]
    fn shared_index_wraps_provider() {
        let provider = symlens::commands::IndexProvider::from_single(
            PathBuf::from("/tmp/test"),
            symlens::model::project::ProjectIndex::new(PathBuf::from("/tmp/test")),
        );

        let shared: Arc<RwLock<symlens::commands::IndexProvider>> = Arc::new(RwLock::new(provider));

        // Read lock works
        let guard = shared.read();
        assert!(!guard.is_workspace());
        assert_eq!(guard.file_count(), 0);
        drop(guard);

        // Write lock works — can swap the provider
        let new_provider = symlens::commands::IndexProvider::from_single(
            PathBuf::from("/tmp/test2"),
            symlens::model::project::ProjectIndex::new(PathBuf::from("/tmp/test2")),
        );
        {
            let mut guard = shared.write();
            *guard = new_provider;
        }

        let guard = shared.read();
        assert_eq!(guard.file_count(), 0);
    }

    /// Verify socket_path produces correct paths.
    #[test]
    fn socket_path_format() {
        let path = symlens::daemon::socket_path("abc123", false);
        assert!(path.to_string_lossy().ends_with("abc123.sock"));

        let ws_path = symlens::daemon::socket_path("abc123", true);
        assert!(ws_path.to_string_lossy().ends_with("ws_abc123.sock"));
    }

    /// Verify RPC request parsing handles malformed input.
    #[test]
    fn rpc_malformed_request() {
        let index = Arc::new(RwLock::new(symlens::commands::IndexProvider::from_single(
            PathBuf::from("/tmp/test"),
            symlens::model::project::ProjectIndex::new(PathBuf::from("/tmp/test")),
        )));

        let resp = symlens::daemon::rpc::handle_request("not json", &index);
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(parsed["error"]["code"], -32700);
    }

    /// Verify RPC response has valid JSON-RPC structure.
    #[test]
    fn rpc_response_structure() {
        let index = Arc::new(RwLock::new(symlens::commands::IndexProvider::from_single(
            PathBuf::from("/tmp/test"),
            symlens::model::project::ProjectIndex::new(PathBuf::from("/tmp/test")),
        )));

        let line = r#"{"jsonrpc":"2.0","id":42,"method":"status","params":{}}"#;
        let resp = symlens::daemon::rpc::handle_request(line, &index);
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();

        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 42);
        assert!(parsed["result"].is_object());
        assert!(parsed.get("error").is_none());
    }

    /// Verify concurrent read locks don't block each other.
    #[test]
    fn shared_index_concurrent_reads() {
        let provider = symlens::commands::IndexProvider::from_single(
            PathBuf::from("/tmp/test"),
            symlens::model::project::ProjectIndex::new(PathBuf::from("/tmp/test")),
        );
        let shared = Arc::new(RwLock::new(provider));

        let r1 = shared.read();
        let r2 = shared.read(); // Should not deadlock
        assert_eq!(r1.file_count(), r2.file_count());
    }

    /// End-to-end test: daemon serves queries over Unix socket.
    #[test]
    fn daemon_e2e_socket_query() {
        use std::io::{BufRead, BufReader, Write};
        use std::os::unix::net::{UnixListener, UnixStream};
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::thread;
        use std::time::Duration;

        let tmp_dir = std::env::temp_dir().join("symlens_test_daemon_e2e");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let sock_path = tmp_dir.join("test.sock");
        if sock_path.exists() {
            std::fs::remove_file(&sock_path).unwrap();
        }

        let index = Arc::new(RwLock::new(symlens::commands::IndexProvider::from_single(
            tmp_dir.clone(),
            symlens::model::project::ProjectIndex::new(tmp_dir.clone()),
        )));
        let shutdown = Arc::new(AtomicBool::new(false));

        // Mini daemon: accept one connection, handle one request, shut down
        let listener = UnixListener::bind(&sock_path).unwrap();
        let index_clone = index.clone();
        let shutdown_clone = shutdown.clone();
        let sock_path_clone = sock_path.clone();

        let handle = thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            for _ in 0..100 {
                if shutdown_clone.load(Ordering::SeqCst) {
                    break;
                }
                match listener.accept() {
                    Ok((stream, _)) => {
                        let mut reader = BufReader::new(stream);
                        let mut line = String::new();
                        if reader.read_line(&mut line).ok() == Some(0) {
                            continue;
                        }
                        let response =
                            symlens::daemon::rpc::handle_request(line.trim_end(), &index_clone);
                        let stream = reader.get_mut();
                        let _ = writeln!(stream, "{}", response);
                        let _ = stream.flush();
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
            let _ = std::fs::remove_file(&sock_path_clone);
        });

        // Client: connect and send status query
        thread::sleep(Duration::from_millis(50));
        let mut stream = UnixStream::connect(&sock_path).unwrap();
        let request = r#"{"jsonrpc":"2.0","id":1,"method":"status","params":{}}"#;
        writeln!(stream, "{}", request).unwrap();
        stream.flush().unwrap();

        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&response_line).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
        assert_eq!(parsed["result"]["is_workspace"], false);
        assert!(parsed["result"]["pid"].is_number());

        shutdown.store(true, Ordering::SeqCst);
        handle.join().unwrap();
    }

    /// Verify is_source_file recognizes all extensions registered in GLOBAL_REGISTRY.
    #[test]
    fn is_source_file_all_extensions() {
        let source_exts = &[
            "rs", "ts", "tsx", "js", "jsx", "mts", "cts", "py", "swift", "go", "dart", "c", "h",
            "cpp", "cc", "cxx", "hpp", "hh", "kt", "kts", "vue",
        ];
        for ext in source_exts {
            let fname = format!("test.{ext}");
            let p = std::path::Path::new(&fname);
            assert!(
                symlens::parser::traits::is_source_file(p),
                "Expected .{ext} to be recognized as source file",
            );
        }

        // Non-source files must be rejected.
        for non in &["md", "toml", "json", "lock"] {
            let fname = format!("test.{non}");
            assert!(
                !symlens::parser::traits::is_source_file(std::path::Path::new(&fname)),
                "Expected .{non} to be rejected",
            );
        }
    }
}
