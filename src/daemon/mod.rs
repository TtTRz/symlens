pub mod client;
pub mod rpc;
pub mod socket;

use crate::commands::IndexProvider;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Thread-safe shared index: watcher takes write lock, socket threads take read lock.
pub type SharedIndex = Arc<RwLock<IndexProvider>>;

/// Socket directory: ~/.symlens/daemon/
pub fn socket_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    home.join(".symlens").join("daemon")
}

/// Compute socket path from project hash.
/// Single-root: `{hash}.sock`, Workspace: `ws_{hash}.sock`.
pub fn socket_path(hash: &str, is_workspace: bool) -> PathBuf {
    let name = if is_workspace {
        format!("ws_{}.sock", hash)
    } else {
        format!("{}.sock", hash)
    };
    socket_dir().join(name)
}
