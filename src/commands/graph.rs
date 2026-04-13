use crate::cli::{GraphArgs, GraphCommand};
use crate::graph::{deps::DepsGraph, impact, path as graph_path};
use crate::index::storage;
use crate::parser::registry::LanguageRegistry;
use std::path::PathBuf;

pub fn run(args: GraphArgs, root_override: Option<&str>, json: bool) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;
    match args.command {
        GraphCommand::Impact(a) => run_impact(a, &root, json),
        GraphCommand::Deps(a) => run_deps(a, &root, json),
        GraphCommand::Path(a) => run_path(a, &root, json),
    }
}

fn run_impact(
    args: crate::cli::GraphImpactArgs,
    root: &std::path::Path,
    json: bool,
) -> anyhow::Result<()> {
    let index = load_index(root)?;
    let graph = index
        .call_graph
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No call graph. Re-run `symlens index`."))?;

    let result = impact::analyze_impact(graph, &args.name, args.depth);

    if json {
        println!(
            "{}",
            serde_json::json!({
                "target": result.target,
                "direct_callers": result.direct_callers,
                "direct_callees": result.direct_callees,
                "transitive_callers": result.transitive_callers.iter()
                    .map(|(n, d)| serde_json::json!({"name": n, "depth": d}))
                    .collect::<Vec<_>>(),
                "transitive_callees": result.transitive_callees.iter()
                    .map(|(n, d)| serde_json::json!({"name": n, "depth": d}))
                    .collect::<Vec<_>>(),
                "total_dependents": result.direct_callers.len() + result.transitive_callers.len(),
                "affected_modules": result.affected_modules,
                "has_cycle": result.has_cycle,
                "risk_score": format!("{:.2}", result.risk_score),
            })
        );
        return Ok(());
    }

    println!("Impact: {}", result.target);
    if result.has_cycle {
        println!("  !! CYCLE DETECTED — this symbol is part of a circular call chain");
    }
    println!("  Risk score: {:.0}%", result.risk_score * 100.0);
    println!();

    if result.direct_callers.is_empty() {
        println!("DIRECT CALLERS: none");
    } else {
        println!("DIRECT CALLERS ({}):", result.direct_callers.len());
        for caller in &result.direct_callers {
            println!("  {}", caller);
        }
    }
    println!();

    if result.direct_callees.is_empty() {
        println!("DIRECT CALLEES: none");
    } else {
        println!("DIRECT CALLEES ({}):", result.direct_callees.len());
        for callee in &result.direct_callees {
            println!("  {}", callee);
        }
    }
    println!();

    if !result.transitive_callers.is_empty() {
        println!(
            "TRANSITIVE (depth={}, {} symbols):",
            args.depth,
            result.transitive_callers.len()
        );
        for (name, depth) in &result.transitive_callers {
            println!("  {} (depth={})", name, depth);
        }
        println!();
    }

    let total = result.direct_callers.len() + result.transitive_callers.len();
    println!(
        "Summary: {} direct callers, {} total dependents, {} modules affected.",
        result.direct_callers.len(),
        total,
        result.affected_modules.len(),
    );

    Ok(())
}

fn run_deps(
    args: crate::cli::GraphDepsArgs,
    root: &std::path::Path,
    _json: bool,
) -> anyhow::Result<()> {
    let index = load_index(root)?;
    let registry = LanguageRegistry::new();

    let mut imports: Vec<(PathBuf, String)> = Vec::new();
    let known_files: Vec<PathBuf> = index.file_symbols.keys().cloned().collect();

    for file_path in &known_files {
        if let Some(ref scope) = args.path
            && !file_path.to_string_lossy().starts_with(scope.as_str())
        {
            continue;
        }

        let full_path = root.join(file_path);
        if let Some(parser) = registry.parser_for(&full_path)
            && let Ok(source) = std::fs::read(&full_path)
            && let Ok(imps) = parser.extract_imports(&source, file_path)
        {
            for imp in imps {
                imports.push((file_path.clone(), imp.module_path));
            }
        }
    }

    let deps_graph = DepsGraph::build(&imports, &known_files);

    if args.fmt == "mermaid" {
        println!("{}", deps_graph.to_mermaid());
    } else if deps_graph.edges.is_empty() {
        println!("No module dependencies found.");
    } else {
        println!("Module dependencies:");
        for (file, deps) in &deps_graph.edges {
            let from = file
                .with_extension("")
                .to_string_lossy()
                .replace("src/", "");
            for dep in deps {
                let to = dep.with_extension("").to_string_lossy().replace("src/", "");
                println!("  {} -> {}", from, to);
            }
        }
    }

    Ok(())
}

fn run_path(
    args: crate::cli::GraphPathArgs,
    root: &std::path::Path,
    json: bool,
) -> anyhow::Result<()> {
    let index = load_index(root)?;
    let graph = index
        .call_graph
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No call graph. Re-run `symlens index`."))?;

    match graph_path::find_path(graph, &args.from, &args.to) {
        Some(path) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "from": args.from, "to": args.to, "path": path, "hops": path.len() - 1 })
                );
                return Ok(());
            }
            println!(
                "Path: {} -> {} ({} hops)",
                args.from,
                args.to,
                path.len() - 1,
            );
            println!();
            for (i, node) in path.iter().enumerate() {
                let indent = "  ".repeat(i);
                let arrow = if i > 0 { "-> " } else { "" };
                println!("{}{}{}", indent, arrow, node);
            }
        }
        None => {
            println!(
                "No path found between \"{}\" and \"{}\"",
                args.from, args.to
            );
        }
    }

    Ok(())
}

fn load_index(root: &std::path::Path) -> anyhow::Result<crate::model::project::ProjectIndex> {
    storage::load(root)?
        .ok_or_else(|| anyhow::anyhow!("No index found. Run `symlens index` first."))
}
