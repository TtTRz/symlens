use serde::Deserialize;
use std::path::Path;

/// Project-level configuration loaded from `symlens.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Maximum number of files to index (default: 100_000)
    pub max_files: usize,
    /// Extra glob patterns to ignore during indexing
    pub ignore: Vec<String>,
    /// Restrict indexing to specific languages (e.g. ["rust", "typescript"])
    /// If empty/not set, all supported languages are indexed.
    pub languages: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_files: 100_000,
            ignore: Vec::new(),
            languages: Vec::new(),
        }
    }
}

impl Config {
    /// Load config from `symlens.toml` in the given directory.
    /// Returns default config if file doesn't exist.
    pub fn load(root: &Path) -> Self {
        let config_path = root.join("symlens.toml");
        if !config_path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(&config_path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("warning: failed to parse symlens.toml: {e}");
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }
}

/// Generate a default `symlens.toml` content with comments.
pub fn default_toml() -> &'static str {
    r#"# SymLens configuration
# Place this file at your project root as `symlens.toml`

# Maximum number of files to index (default: 100000)
# max_files = 100000

# Extra glob patterns to ignore (in addition to .gitignore)
# ignore = ["vendor/**", "node_modules/**", "*.generated.go"]

# Restrict indexing to specific languages (default: all supported)
# Supported: rust, typescript, python, swift, go, dart, c, cpp, kotlin
# languages = ["rust", "typescript"]
"#
}

/// Workspace-level configuration loaded from `symlens.workspace.toml`.
/// Enables multi-root indexing by declaring workspace member directories.
///
/// Example `symlens.workspace.toml`:
/// ```toml
/// [workspace]
/// roots = ["../core", "../plugins/audio", "../plugins/video"]
///
/// [workspace.defaults]
/// max_files = 50000
/// ignore = ["vendor/**"]
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceConfig {
    pub workspace: WorkspaceSection,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceSection {
    /// List of workspace root directories (relative to the config file location).
    pub roots: Vec<String>,
    /// Default configuration applied to all roots unless overridden by per-root symlens.toml.
    #[serde(default)]
    pub defaults: Config,
}

impl WorkspaceConfig {
    /// Load workspace config from `symlens.workspace.toml` in the given directory.
    /// Returns None if the file doesn't exist.
    pub fn load(dir: &Path) -> Option<Self> {
        let config_path = dir.join("symlens.workspace.toml");
        if !config_path.exists() {
            return None;
        }
        match std::fs::read_to_string(&config_path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(config) => Some(config),
                Err(e) => {
                    eprintln!("warning: failed to parse symlens.workspace.toml: {e}");
                    None
                }
            },
            Err(_) => None,
        }
    }

    /// Resolve workspace root paths to absolute paths.
    /// Relative paths are resolved relative to `config_dir`.
    pub fn resolve_roots(&self, config_dir: &Path) -> Vec<std::path::PathBuf> {
        self.workspace
            .roots
            .iter()
            .filter_map(|root_str| {
                let path = std::path::PathBuf::from(root_str);
                let abs_path = if path.is_absolute() {
                    path
                } else {
                    config_dir.join(&path)
                };
                abs_path.canonicalize().ok()
            })
            .collect()
    }
}

/// Generate a default `symlens.workspace.toml` content with comments.
pub fn default_workspace_toml() -> &'static str {
    r#"# SymLens workspace configuration
# Place this file at your workspace root as `symlens.workspace.toml`

[workspace]
# List of project root directories (relative to this file)
roots = []

# Default settings applied to all roots (overridable per-root via symlens.toml)
# [workspace.defaults]
# max_files = 50000
# ignore = ["vendor/**"]
"#
}
