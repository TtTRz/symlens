use serde::Deserialize;
use std::path::Path;

/// Project-level configuration loaded from `symlens.toml`.
#[derive(Debug, Deserialize)]
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
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
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
