use crate::cli::CallersArgs;

pub fn run_callers(
    args: CallersArgs,
    root_override: Option<&str>,
    workspace_flag: bool,
    json: bool,
) -> anyhow::Result<()> {
    let provider = crate::commands::IndexProvider::load(root_override, workspace_flag)?;
    let graph = provider
        .call_graph()
        .ok_or_else(|| anyhow::anyhow!("No call graph in index. Re-run `symlens index`."))?;

    let callers = graph.callers(&args.name);

    if json {
        let items: Vec<_> = callers.iter().take(args.limit).collect();
        println!(
            "{}",
            serde_json::json!({ "symbol": args.name, "callers": items, "count": callers.len() })
        );
        return Ok(());
    }

    if callers.is_empty() {
        println!("No callers found for \"{}\"", args.name);
    } else {
        println!("Callers of {} ({}):", args.name, callers.len());
        for caller in callers.iter().take(args.limit) {
            println!("  {}", caller);
        }
    }
    Ok(())
}

pub fn run_callees(
    args: CallersArgs,
    root_override: Option<&str>,
    workspace_flag: bool,
    json: bool,
) -> anyhow::Result<()> {
    let provider = crate::commands::IndexProvider::load(root_override, workspace_flag)?;
    let graph = provider
        .call_graph()
        .ok_or_else(|| anyhow::anyhow!("No call graph in index. Re-run `symlens index`."))?;

    let callees = graph.callees(&args.name);

    if json {
        let items: Vec<_> = callees.iter().take(args.limit).collect();
        println!(
            "{}",
            serde_json::json!({ "symbol": args.name, "callees": items, "count": callees.len() })
        );
        return Ok(());
    }

    if callees.is_empty() {
        println!("No callees found for \"{}\"", args.name);
    } else {
        println!("Callees of {} ({}):", args.name, callees.len());
        for callee in callees.iter().take(args.limit) {
            println!("  {}", callee);
        }
    }
    Ok(())
}
