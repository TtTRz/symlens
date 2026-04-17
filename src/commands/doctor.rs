use crate::index::storage;
use crate::parser::registry::LanguageRegistry;
use std::path::Path;

pub fn run(root_override: Option<&str>, workspace_flag: bool) -> anyhow::Result<()> {
    let provider = crate::commands::IndexProvider::load(root_override, workspace_flag)?;

    println!("SymLens Doctor");
    println!("===============");

    if let Some(root) = provider.single_root() {
        println!("Project: {}", root.display());
    } else {
        println!("Project: workspace ({} roots)", provider.roots().len());
    }
    println!();

    // 1. Check index
    check_index(&provider);

    // 2. Check cache disk usage (per root)
    check_cache(&provider);

    // 3. Check supported languages in project
    check_languages(&provider);

    // 4. Check call graph
    check_call_graph(&provider);

    // 5. Check search engine (per root)
    check_search(&provider);

    println!();
    println!("Run `symlens index` to rebuild if issues are found.");
    Ok(())
}

fn check_index(provider: &crate::commands::IndexProvider) {
    let indexed_at = provider.indexed_at();
    let age_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub(indexed_at);

    let age_str = if age_secs < 60 {
        format!("{}s ago", age_secs)
    } else if age_secs < 3600 {
        format!("{}m ago", age_secs / 60)
    } else if age_secs < 86400 {
        format!("{}h ago", age_secs / 3600)
    } else {
        format!("{}d ago", age_secs / 86400)
    };

    let files = provider.file_count();
    let symbols = provider.symbols().len();

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

fn check_cache(provider: &crate::commands::IndexProvider) {
    for (_, _root_path, root_hash) in provider.roots() {
        let cache_dir = storage::cache_dir(root_hash);

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
            if provider.is_workspace() {
                println!(
                    "  \u{2713} Cache [{}]: {} at {}",
                    root_hash,
                    size_str,
                    cache_dir.display()
                );
            } else {
                println!("  \u{2713} Cache: {} at {}", size_str, cache_dir.display());
            }
        } else if provider.is_workspace() {
            println!("  \u{2717} Cache [{}]: not found", root_hash);
        } else {
            println!("  \u{2717} Cache: not found");
        }
    }
}

fn check_languages(provider: &crate::commands::IndexProvider) {
    let registry = LanguageRegistry::new();
    let extensions = [
        "rs", "ts", "tsx", "js", "py", "swift", "go", "dart", "c", "h", "cpp", "cc", "cxx", "hpp",
        "kt", "kts",
    ];
    let mut found: Vec<(&str, usize)> = Vec::new();

    for (_, root_path, _) in provider.roots() {
        for ext in &extensions {
            let count = count_files_with_ext(root_path, ext);
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
                // Merge counts for same language across roots
                if let Some(entry) = found.iter_mut().find(|(l, _)| *l == lang) {
                    entry.1 += count;
                } else {
                    found.push((lang, count));
                }
            }
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

fn check_call_graph(provider: &crate::commands::IndexProvider) {
    if let Some(cg) = provider.call_graph() {
        println!(
            "  \u{2713} Call graph: {} nodes, {} edges",
            cg.nodes.len(),
            cg.edges.len()
        );
    } else {
        println!("  \u{26a0} Call graph: not built (no call edges extracted)");
    }
}

fn check_search(provider: &crate::commands::IndexProvider) {
    for (_, root_path, _) in provider.roots() {
        match storage::open_search(root_path) {
            Ok(Some(_)) => {
                if provider.is_workspace() {
                    println!("  \u{2713} Search engine [{}]: ready", root_path.display());
                } else {
                    println!("  \u{2713} Search engine: ready");
                }
            }
            Ok(None) => {
                if provider.is_workspace() {
                    println!(
                        "  \u{2717} Search engine [{}]: not built",
                        root_path.display()
                    );
                } else {
                    println!("  \u{2717} Search engine: not built");
                }
            }
            Err(e) => {
                if provider.is_workspace() {
                    println!(
                        "  \u{2717} Search engine [{}]: error — {}",
                        root_path.display(),
                        e
                    );
                } else {
                    println!("  \u{2717} Search engine: error — {}", e);
                }
            }
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
