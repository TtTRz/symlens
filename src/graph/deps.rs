use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Module dependency graph built from import/use statements.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DepsGraph {
    /// file → list of files it imports from
    pub edges: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
}

impl DepsGraph {
    /// Build from (importing_file, imported_module_path) pairs.
    pub fn build(imports: &[(PathBuf, String)], known_files: &[PathBuf]) -> Self {
        let mut graph = DepsGraph::default();

        for (file, module_path) in imports {
            // Try to resolve module_path to a known file
            if let Some(target) = resolve_module(module_path, known_files)
                && &target != file
            {
                graph.edges.entry(file.clone()).or_default().insert(target);
            }
        }

        graph
    }

    /// Get files that depend on the given file (reverse deps).
    #[allow(dead_code)]
    pub fn dependents(&self, file: &PathBuf) -> Vec<&PathBuf> {
        self.edges
            .iter()
            .filter(|(_, deps)| deps.contains(file))
            .map(|(f, _)| f)
            .collect()
    }

    /// Get files that the given file depends on.
    #[allow(dead_code)]
    pub fn dependencies(&self, file: &PathBuf) -> Vec<&PathBuf> {
        self.edges
            .get(file)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// Format as Mermaid graph.
    pub fn to_mermaid(&self) -> String {
        let mut out = String::from("graph TD\n");
        for (file, deps) in &self.edges {
            let from = module_name(file);
            for dep in deps {
                let to = module_name(dep);
                out.push_str(&format!("    {} --> {}\n", from, to));
            }
        }
        out
    }
}

/// Try to resolve a module path to a known file.
/// Supports multiple language conventions:
///   Rust:   "crate::audio::engine" → "src/audio/engine.rs"
///   C/C++:  "stdio.h" or "foo/bar.h" → direct path match
///   Python: "foo.bar" → "foo/bar.py" or "foo/bar/__init__.py"
///   Go:     "fmt" or "pkg/foo" → match by last path segment
///   Kotlin: "com.example.Foo" → "com/example/Foo.kt"
///   TS/JS:  "./foo" → "foo.ts" or "foo/index.ts"
fn resolve_module(module_path: &str, known_files: &[PathBuf]) -> Option<PathBuf> {
    // Direct file path match (C/C++ #include "foo/bar.h")
    let direct = PathBuf::from(module_path);
    if known_files.contains(&direct) {
        return Some(direct);
    }

    // Rust: strip crate:: prefix
    let cleaned = module_path.replace("crate::", "src/").replace("::", "/");
    let candidates = [format!("{}.rs", cleaned), format!("{}/mod.rs", cleaned)];
    for candidate in &candidates {
        let p = PathBuf::from(candidate);
        if known_files.contains(&p) {
            return Some(p);
        }
    }

    // Python: foo.bar → foo/bar.py or foo/bar/__init__.py
    if module_path.contains('.') && !module_path.contains('/') && !module_path.contains("::") {
        let py_path = module_path.replace('.', "/");
        let py_candidates = [
            format!("{}.py", py_path),
            format!("{}/__init__.py", py_path),
            format!("{}.kt", py_path), // also works for Kotlin
        ];
        for candidate in &py_candidates {
            let p = PathBuf::from(candidate);
            if known_files.contains(&p) {
                return Some(p);
            }
        }
    }

    // TS/JS: strip leading ./ and try extensions
    if module_path.starts_with("./") || module_path.starts_with("../") {
        let cleaned = module_path.trim_start_matches("./");
        let ts_candidates = [
            format!("{}.ts", cleaned),
            format!("{}.tsx", cleaned),
            format!("{}.js", cleaned),
            format!("{}/index.ts", cleaned),
        ];
        for candidate in &ts_candidates {
            let p = PathBuf::from(candidate);
            if known_files.contains(&p) {
                return Some(p);
            }
        }
    }

    // Fuzzy: match last segment against file stems
    let last_segment = module_path
        .rsplit("::")
        .next()
        .or_else(|| module_path.rsplit('/').next())
        .or_else(|| module_path.rsplit('.').next())?;

    for file in known_files {
        let stem = file.file_stem()?.to_str()?;
        if stem == last_segment {
            return Some(file.clone());
        }
    }

    None
}

fn module_name(path: &std::path::Path) -> String {
    path.with_extension("")
        .to_string_lossy()
        .replace('/', "::")
        .replace("src::", "")
}
