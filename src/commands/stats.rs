use crate::cli::StatsArgs;

pub fn run(
    _args: StatsArgs,
    root_override: Option<&str>,
    workspace_flag: bool,
    json: bool,
) -> anyhow::Result<()> {
    let provider = crate::commands::IndexProvider::load(root_override, workspace_flag)?;

    let stats = provider.stats();

    let root_display = provider
        .single_root()
        .map(|r| r.to_string_lossy().into_owned())
        .unwrap_or_else(|| "workspace".to_string());

    if json {
        println!(
            "{}",
            serde_json::json!({
                "root": root_display,
                "version": provider.version(),
                "files": stats.total_files,
                "symbols": stats.total_symbols,
                "by_language": stats.by_language,
                "by_kind": stats.by_kind,
            })
        );
        return Ok(());
    }

    println!("Project: {}", root_display);
    println!("Index version: {}", provider.version());
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
