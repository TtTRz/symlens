use crate::cli::{GraphArgs, GraphCommand};
use crate::graph::{deps::DepsGraph, impact, path as graph_path};
use crate::index::storage;
use crate::parser::registry::LanguageRegistry;
use std::path::PathBuf;

pub fn run(args: GraphArgs) -> anyhow::Result<()> {
    match args.command {
        GraphCommand::Impact(a) => run_impact(a),
        GraphCommand::Deps(a) => run_deps(a),
        GraphCommand::Path(a) => run_path(a),
    }
}

fn run_impact(args: crate::cli::GraphImpactArgs) -> anyhow::Result<()> {
    let root = resolve_root()?;
    let index = load_index(&root)?;
    let graph = index
        .call_graph
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No call graph. Re-run `codelens index`."))?;

    let result = impact::analyze_impact(graph, &args.name, args.depth);

    println!("Impact: {}", result.target);
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
        "⚠️  {} direct callers, {} total dependents.",
        result.direct_callers.len(),
        total,
    );

    Ok(())
}

fn run_deps(args: crate::cli::GraphDepsArgs) -> anyhow::Result<()> {
    let root = resolve_root()?;
    let index = load_index(&root)?;
    let registry = LanguageRegistry::new();

    // Collect use/import statements from all files
    let mut imports: Vec<(PathBuf, String)> = Vec::new();
    let known_files: Vec<PathBuf> = index.file_symbols.keys().cloned().collect();

    for file_path in &known_files {
        // Apply path filter
        if let Some(ref scope) = args.path {
            if !file_path.to_string_lossy().starts_with(scope.as_str()) {
                continue;
            }
        }

        let full_path = root.join(file_path);
        if let Some(parser) = registry.parser_for(&full_path) {
            if let Ok(source) = std::fs::read(&full_path) {
                // Extract use/import statements by scanning for "use " patterns
                if let Ok(source_str) = std::str::from_utf8(&source) {
                    for line in source_str.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("use ") {
                            // Parse: "use crate::audio::engine::AudioEngine;"
                            let module = trimmed
                                .trim_start_matches("use ")
                                .split('{')
                                .next()
                                .unwrap_or("")
                                .trim_end_matches(';')
                                .trim_end_matches("::")
                                .trim();
                            if !module.is_empty() {
                                imports.push((file_path.clone(), module.to_string()));
                            }
                        }
                    }
                }
                let _ = parser; // suppress unused
            }
        }
    }

    let deps_graph = DepsGraph::build(&imports, &known_files);

    if args.fmt == "mermaid" {
        println!("{}", deps_graph.to_mermaid());
    } else {
        // Text format
        if deps_graph.edges.is_empty() {
            println!("No module dependencies found.");
        } else {
            println!("Module dependencies:");
            for (file, deps) in &deps_graph.edges {
                let from = file.with_extension("").to_string_lossy().replace("src/", "");
                for dep in deps {
                    let to = dep.with_extension("").to_string_lossy().replace("src/", "");
                    println!("  {} → {}", from, to);
                }
            }
        }
    }

    Ok(())
}

fn run_path(args: crate::cli::GraphPathArgs) -> anyhow::Result<()> {
    let root = resolve_root()?;
    let index = load_index(&root)?;
    let graph = index
        .call_graph
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No call graph. Re-run `codelens index`."))?;

    match graph_path::find_path(graph, &args.from, &args.to) {
        Some(path) => {
            println!(
                "Path: {} → {} ({} hops)",
                args.from,
                args.to,
                path.len() - 1,
            );
            println!();
            for (i, node) in path.iter().enumerate() {
                let indent = "  ".repeat(i);
                let arrow = if i > 0 { "└→ " } else { "" };
                println!("{}{}{}", indent, arrow, node);
            }
        }
        None => {
            println!("No path found between \"{}\" and \"{}\"", args.from, args.to);
        }
    }

    Ok(())
}

fn resolve_root() -> anyhow::Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    Ok(storage::find_project_root(&cwd).unwrap_or(cwd))
}

fn load_index(root: &PathBuf) -> anyhow::Result<crate::model::project::ProjectIndex> {
    storage::load(root)?
        .ok_or_else(|| anyhow::anyhow!("No index found. Run `codelens index` first."))
}
