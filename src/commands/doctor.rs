use crate::index::storage;
use crate::parser::registry::LanguageRegistry;
use std::path::Path;

pub fn run(root_override: Option<&str>) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;
    println!("CodeLens Doctor");
    println!("===============");
    println!("Project: {}", root.display());
    println!();

    // 1. Check index
    check_index(&root);

    // 2. Check cache disk usage
    check_cache(&root);

    // 3. Check supported languages in project
    check_languages(&root);

    // 4. Check call graph
    check_call_graph(&root);

    // 5. Check search engine
    check_search(&root);

    println!();
    println!("Run `codelens index` to rebuild if issues are found.");
    Ok(())
}

fn check_index(root: &Path) {
    match storage::load(root) {
        Ok(Some(index)) => {
            let age_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .saturating_sub(index.indexed_at);

            let age_str = if age_secs < 60 {
                format!("{}s ago", age_secs)
            } else if age_secs < 3600 {
                format!("{}m ago", age_secs / 60)
            } else if age_secs < 86400 {
                format!("{}h ago", age_secs / 3600)
            } else {
                format!("{}d ago", age_secs / 86400)
            };

            let files = index.file_symbols.len();
            let symbols = index.symbols.len();

            if age_secs > 86400 {
                println!(
                    "  \u{26a0} Index: {} files, {} symbols (stale — indexed {})",
                    files, symbols, age_str
                );
            } else {
                println!(
                    "  \u{2713} Index: {} files, {} symbols (indexed {})",
                    files, symbols, age_str
                );
            }
        }
        Ok(None) => {
            println!("  \u{2717} Index: not found — run `codelens index`");
        }
        Err(e) => {
            println!("  \u{2717} Index: error loading — {}", e);
        }
    }
}

fn check_cache(root: &Path) {
    let root_hash = blake3::hash(root.to_string_lossy().as_bytes()).to_hex()[..16].to_string();
    let cache_dir = storage::cache_dir(&root_hash);

    if cache_dir.exists() {
        let mut total_bytes: u64 = 0;
        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    total_bytes += meta.len();
                }
                // Also check subdirectories (search/)
                if entry.path().is_dir()
                    && let Ok(sub_entries) = std::fs::read_dir(entry.path())
                {
                    for sub in sub_entries.flatten() {
                        if let Ok(meta) = sub.metadata() {
                            total_bytes += meta.len();
                        }
                    }
                }
            }
        }
        let size_str = if total_bytes < 1024 {
            format!("{} B", total_bytes)
        } else if total_bytes < 1024 * 1024 {
            format!("{:.1} KB", total_bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", total_bytes as f64 / (1024.0 * 1024.0))
        };
        println!("  \u{2713} Cache: {} at {}", size_str, cache_dir.display());
    } else {
        println!("  \u{2717} Cache: not found");
    }
}

fn check_languages(root: &Path) {
    let registry = LanguageRegistry::new();
    let extensions = [
        "rs", "ts", "tsx", "js", "py", "swift", "go", "dart", "c", "h", "cpp", "cc", "cxx", "hpp",
        "kt", "kts",
    ];
    let mut found: Vec<(&str, usize)> = Vec::new();

    for ext in &extensions {
        let count = count_files_with_ext(root, ext);
        if count > 0 {
            let lang = match *ext {
                "rs" => "Rust",
                "ts" | "tsx" => "TypeScript",
                "js" => "JavaScript",
                "py" => "Python",
                "swift" => "Swift",
                "go" => "Go",
                "dart" => "Dart",
                "c" | "h" => "C",
                "cpp" | "cc" | "cxx" | "hpp" => "C++",
                "kt" | "kts" => "Kotlin",
                _ => ext,
            };
            found.push((lang, count));
        }
    }
    let _ = registry;

    if found.is_empty() {
        println!("  \u{26a0} Languages: no supported source files found");
    } else {
        let langs: Vec<String> = found
            .iter()
            .map(|(l, c)| format!("{} ({} files)", l, c))
            .collect();
        println!("  \u{2713} Languages: {}", langs.join(", "));
    }
}

fn check_call_graph(root: &Path) {
    match storage::load(root) {
        Ok(Some(index)) => {
            if let Some(ref cg) = index.call_graph {
                println!(
                    "  \u{2713} Call graph: {} nodes, {} edges",
                    cg.nodes.len(),
                    cg.edges.len()
                );
            } else {
                println!("  \u{26a0} Call graph: not built (no call edges extracted)");
            }
        }
        _ => {
            println!("  \u{2717} Call graph: no index");
        }
    }
}

fn check_search(root: &Path) {
    match storage::open_search(root) {
        Ok(Some(_)) => {
            println!("  \u{2713} Search engine: ready");
        }
        Ok(None) => {
            println!("  \u{2717} Search engine: not built");
        }
        Err(e) => {
            println!("  \u{2717} Search engine: error — {}", e);
        }
    }
}

fn count_files_with_ext(root: &Path, ext: &str) -> usize {
    ignore::WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
        .filter(|e| e.path().extension().and_then(|e| e.to_str()) == Some(ext))
        .count()
}
