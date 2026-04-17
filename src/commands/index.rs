use crate::cli::IndexArgs;
use crate::commands::IndexProvider;
use crate::config::WorkspaceConfig;
use crate::index::{indexer, storage};
use crate::model::project::RootInfo;
use std::path::PathBuf;

pub fn run(
    args: IndexArgs,
    root_override: Option<&str>,
    workspace_flag: bool,
) -> anyhow::Result<()> {
    let root = match args.path {
        Some(p) => PathBuf::from(p).canonicalize()?,
        None => crate::commands::resolve_root(root_override)?,
    };

    // Check for workspace config
    let ws_config = WorkspaceConfig::load(&root);

    if workspace_flag || ws_config.is_some() {
        // Workspace mode
        let config = ws_config.ok_or_else(|| {
            anyhow::anyhow!(
                "Workspace mode requested but no symlens.workspace.toml found in {}",
                root.display()
            )
        })?;

        let root_paths = config.resolve_roots(&root);
        if root_paths.is_empty() {
            anyhow::bail!("No valid roots found in symlens.workspace.toml");
        }

        let roots: Vec<RootInfo> = root_paths
            .iter()
            .map(|p| {
                let mut info = RootInfo::new(p.clone());
                info.config = crate::config::Config::load(p);
                if info.config.max_files == 100_000
                    && config.workspace.defaults.max_files != 100_000
                {
                    info.config.max_files = config.workspace.defaults.max_files;
                }
                if info.config.ignore.is_empty() && !config.workspace.defaults.ignore.is_empty() {
                    info.config.ignore = config.workspace.defaults.ignore.clone();
                }
                if info.config.languages.is_empty()
                    && !config.workspace.defaults.languages.is_empty()
                {
                    info.config.languages = config.workspace.defaults.languages.clone();
                }
                info
            })
            .collect();

        if !args.quiet {
            eprintln!("Indexing workspace ({} roots)...", roots.len());
        }

        // Load previous workspace index for incremental mode (unless --force)
        let prev_ws = if args.force {
            None
        } else {
            storage::load_workspace(&roots).ok().flatten()
        };

        let result = indexer::index_workspace(&roots, args.max_files, prev_ws.as_ref())?;
        let cache_path = storage::save_workspace(&result.index)?;

        if args.json {
            let stats = result.index.stats();
            println!(
                "{}",
                serde_json::json!({
                    "roots": roots.iter().map(|r| r.path.to_string_lossy().to_string()).collect::<Vec<_>>(),
                    "files_scanned": result.files_scanned,
                    "files_parsed": result.files_parsed,
                    "files_skipped": result.files_skipped,
                    "symbols": stats.total_symbols,
                    "duration_ms": result.duration_ms,
                    "cache": cache_path.to_string_lossy(),
                    "incremental": result.files_skipped > 0,
                    "by_language": stats.by_language,
                    "by_kind": stats.by_kind,
                })
            );
        } else if args.quiet {
            // Load provider to get symbol count
            let provider = IndexProvider::Workspace {
                index: result.index,
            };
            println!("{}", provider.symbols().len());
        } else {
            let stats = result.index.stats();
            println!("✓ Indexed workspace ({} roots)", roots.len());
            if result.files_skipped > 0 {
                println!(
                    "  Files: {} scanned, {} parsed, \x1b[32m{} unchanged\x1b[0m ({})",
                    result.files_scanned,
                    result.files_parsed,
                    result.files_skipped,
                    format_lang_counts(&stats.by_language)
                );
            } else {
                println!(
                    "  Files: {} scanned, {} parsed ({})",
                    result.files_scanned,
                    result.files_parsed,
                    format_lang_counts(&stats.by_language)
                );
            }
            println!(
                "  Symbols: {} ({})",
                stats.total_symbols,
                format_kind_counts(&stats.by_kind)
            );
            println!("  Time: {}ms", result.duration_ms);
            println!("  Cache: {}", cache_path.display());
        }
    } else {
        // Single-root mode (existing behavior)
        if !args.quiet {
            eprintln!("Indexing {}...", root.display());
        }

        let prev_index = if args.force {
            None
        } else {
            storage::load(&root).ok().flatten()
        };

        let result =
            indexer::index_project_incremental(&root, args.max_files, prev_index.as_ref())?;
        let cache_path = storage::save(&result.index)?;

        if args.json {
            let stats = result.index.stats();
            println!(
                "{}",
                serde_json::json!({
                    "root": root.to_string_lossy(),
                    "files_scanned": result.files_scanned,
                    "files_parsed": result.files_parsed,
                    "files_skipped": result.files_skipped,
                    "symbols": stats.total_symbols,
                    "duration_ms": result.duration_ms,
                    "cache": cache_path.to_string_lossy(),
                    "incremental": result.files_skipped > 0,
                    "by_language": stats.by_language,
                    "by_kind": stats.by_kind,
                })
            );
        } else if args.quiet {
            println!("{}", result.index.symbols.len());
        } else {
            let stats = result.index.stats();
            println!("✓ Indexed {}", root.display());
            if result.files_skipped > 0 {
                println!(
                    "  Files: {} scanned, {} parsed, \x1b[32m{} unchanged\x1b[0m ({})",
                    result.files_scanned,
                    result.files_parsed,
                    result.files_skipped,
                    format_lang_counts(&stats.by_language)
                );
            } else {
                println!(
                    "  Files: {} scanned, {} parsed ({})",
                    result.files_scanned,
                    result.files_parsed,
                    format_lang_counts(&stats.by_language)
                );
            }
            println!(
                "  Symbols: {} ({})",
                stats.total_symbols,
                format_kind_counts(&stats.by_kind)
            );
            println!("  Time: {}ms", result.duration_ms);
            println!("  Cache: {}", cache_path.display());
        }
    }

    Ok(())
}

fn format_lang_counts(counts: &std::collections::HashMap<String, usize>) -> String {
    let mut pairs: Vec<_> = counts.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1));
    pairs
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_kind_counts(counts: &std::collections::HashMap<String, usize>) -> String {
    let mut pairs: Vec<_> = counts.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1));
    pairs
        .iter()
        .take(5)
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect::<Vec<_>>()
        .join(", ")
}
