use crate::cli::IndexArgs;
use crate::index::{indexer, storage};
use std::path::PathBuf;

pub fn run(args: IndexArgs, root_override: Option<&str>) -> anyhow::Result<()> {
    let root = match args.path {
        Some(p) => PathBuf::from(p).canonicalize()?,
        None => crate::commands::resolve_root(root_override)?,
    };

    if !args.quiet {
        eprintln!("Indexing {}...", root.display());
    }

    // Load previous index for incremental mode (unless --force)
    let prev_index = if args.force {
        None
    } else {
        storage::load(&root).ok().flatten()
    };

    let result = indexer::index_project_incremental(&root, args.max_files, prev_index.as_ref())?;
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
