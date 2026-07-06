use crate::cli::CallersArgs;
use crate::output::color;

pub fn run_callers(
    args: CallersArgs,
    root_override: Option<&str>,
    workspace_flag: bool,
    json: bool,
    color_on: bool,
) -> anyhow::Result<()> {
    let provider = crate::commands::IndexProvider::load(root_override, workspace_flag)?;
    let graph = provider
        .call_graph()
        .ok_or_else(|| anyhow::anyhow!("No call graph in index. Re-run `symlens index`."))?;

    let names = graph.callers(&args.name);

    if json {
        let items =
            crate::output::json::enrich_callers_json(&names.to_vec(), args.limit, &provider);
        println!(
            "{}",
            serde_json::json!({ "symbol": args.name, "callers": items, "count": names.len() })
        );
        return Ok(());
    }

    if names.is_empty() {
        println!("No callers found for \"{}\"", args.name);
    } else {
        println!(
            "Callers of {} ({}):",
            color::bold(&args.name, color_on),
            names.len()
        );
        for name in names.iter().take(args.limit) {
            if let Some(sym) = provider.find_symbol(name) {
                let sig = sym.signature.as_deref().unwrap_or(name);
                let sig_display = if sig.chars().count() > 80 {
                    format!("{}...", color::truncate_str(sig, 77))
                } else {
                    sig.to_string()
                };
                println!(
                    "  {} {} {} {}",
                    color::cyan(name, color_on),
                    sig_display,
                    color::dim(
                        &format!("{}:L{}", sym.file_path.display(), sym.span.start_line),
                        color_on,
                    ),
                    color::dim(&format!("({})", sym.kind), color_on),
                );
            } else {
                println!("  {}", name);
            }
        }
    }
    Ok(())
}

pub fn run_callees(
    args: CallersArgs,
    root_override: Option<&str>,
    workspace_flag: bool,
    json: bool,
    color_on: bool,
) -> anyhow::Result<()> {
    let provider = crate::commands::IndexProvider::load(root_override, workspace_flag)?;
    let graph = provider
        .call_graph()
        .ok_or_else(|| anyhow::anyhow!("No call graph in index. Re-run `symlens index`."))?;

    let names = graph.callees(&args.name);

    if json {
        let items =
            crate::output::json::enrich_callers_json(&names.to_vec(), args.limit, &provider);
        println!(
            "{}",
            serde_json::json!({ "symbol": args.name, "callees": items, "count": names.len() })
        );
        return Ok(());
    }

    if names.is_empty() {
        println!("No callees found for \"{}\"", args.name);
    } else {
        println!(
            "Callees of {} ({}):",
            color::bold(&args.name, color_on),
            names.len()
        );
        for name in names.iter().take(args.limit) {
            if let Some(sym) = provider.find_symbol(name) {
                let sig = sym.signature.as_deref().unwrap_or(name);
                let sig_display = if sig.chars().count() > 80 {
                    format!("{}...", color::truncate_str(sig, 77))
                } else {
                    sig.to_string()
                };
                println!(
                    "  {} {} {} {}",
                    color::cyan(name, color_on),
                    sig_display,
                    color::dim(
                        &format!("{}:L{}", sym.file_path.display(), sym.span.start_line),
                        color_on,
                    ),
                    color::dim(&format!("({})", sym.kind), color_on),
                );
            } else {
                println!("  {}", name);
            }
        }
    }
    Ok(())
}
