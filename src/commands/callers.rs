use crate::cli::CallersArgs;
use crate::index::storage;

pub fn run_callers(args: CallersArgs, root_override: Option<&str>) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;

    let index = storage::load(&root)?
        .ok_or_else(|| anyhow::anyhow!("No index found. Run `codelens index` first."))?;

    let graph = index
        .call_graph
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No call graph in index. Re-run `codelens index`."))?;

    let callers = graph.callers(&args.name);

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

pub fn run_callees(args: CallersArgs, root_override: Option<&str>) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;

    let index = storage::load(&root)?
        .ok_or_else(|| anyhow::anyhow!("No index found. Run `codelens index` first."))?;

    let graph = index
        .call_graph
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No call graph in index. Re-run `codelens index`."))?;

    let callees = graph.callees(&args.name);

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
