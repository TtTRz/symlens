use crate::cli::IndexArgs;
use crate::index::{indexer, storage};
use std::path::PathBuf;

pub fn run(args: IndexArgs) -> anyhow::Result<()> {
    let root = match args.path {
        Some(p) => PathBuf::from(p).canonicalize()?,
        None => {
            let cwd = std::env::current_dir()?;
            storage::find_project_root(&cwd).unwrap_or(cwd)
        }
    };

    if !args.quiet {
        eprintln!("Indexing {}...", root.display());
    }

    let result = indexer::index_project(&root, args.max_files)?;
    let cache_path = storage::save(&result.index)?;

    if args.quiet {
        println!("{}", result.index.symbols.len());
    } else {
        let stats = result.index.stats();
        println!("✓ Indexed {}", root.display());
        println!(
            "  Files: {} ({})",
            stats.total_files,
            format_lang_counts(&stats.by_language)
        );
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
