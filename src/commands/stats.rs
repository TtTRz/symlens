use crate::cli::StatsArgs;
use crate::index::storage;

pub fn run(_args: StatsArgs) -> anyhow::Result<()> {
    let root = {
        let cwd = std::env::current_dir()?;
        storage::find_project_root(&cwd).unwrap_or(cwd)
    };

    let index = storage::load(&root)?
        .ok_or_else(|| anyhow::anyhow!("No index found. Run `codelens index` first."))?;

    let stats = index.stats();

    println!("Project: {}", index.root.display());
    println!("Index version: {}", index.version);
    println!("Files: {}", stats.total_files);
    println!("Symbols: {}", stats.total_symbols);
    println!();

    println!("By language:");
    let mut langs: Vec<_> = stats.by_language.iter().collect();
    langs.sort_by(|a, b| b.1.cmp(a.1));
    for (lang, count) in langs {
        println!("  {}: {} symbols", lang, count);
    }
    println!();

    println!("By kind:");
    let mut kinds: Vec<_> = stats.by_kind.iter().collect();
    kinds.sort_by(|a, b| b.1.cmp(a.1));
    for (kind, count) in kinds {
        println!("  {}: {}", kind, count);
    }

    Ok(())
}
