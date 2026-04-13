pub mod blame;
pub mod callers;
pub mod diff;
pub mod export;
pub mod graph;
pub mod index;
pub mod lines;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod outline;
pub mod refs;
pub mod search;
pub mod setup;
pub mod stats;
pub mod symbol;
pub mod watch;

use std::path::PathBuf;

/// Resolve project root: use explicit --root if provided, otherwise auto-detect via .git.
pub fn resolve_root(explicit: Option<&str>) -> anyhow::Result<PathBuf> {
    if let Some(root) = explicit {
        let p = PathBuf::from(root).canonicalize()?;
        return Ok(p);
    }
    let cwd = std::env::current_dir()?;
    Ok(crate::index::storage::find_project_root(&cwd).unwrap_or(cwd))
}
