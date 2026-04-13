use crate::cli::OutlineArgs;
use crate::index::storage;
use crate::model::symbol::{Symbol, SymbolKind};
use crate::output::color;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub fn run(
    args: OutlineArgs,
    root_override: Option<&str>,
    json: bool,
    color_on: bool,
) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;

    let index = storage::load(&root)?
        .ok_or_else(|| anyhow::anyhow!("No index found. Run `codelens index` first."))?;

    if json {
        if args.project || args.file.is_none() {
            let stats = index.stats();
            let files: Vec<serde_json::Value> = index
                .file_symbols
                .iter()
                .map(|(file, ids)| serde_json::json!({ "file": file.to_string_lossy(), "symbols": ids.len() }))
                .collect();
            println!(
                "{}",
                serde_json::json!({
                    "files": files,
                    "total_files": stats.total_files,
                    "total_symbols": stats.total_symbols,
                    "by_language": stats.by_language,
                })
            );
        } else if let Some(ref file) = args.file {
            let file_path = PathBuf::from(file);
            let symbols = index.symbols_in_file(&file_path);
            let items: Vec<serde_json::Value> = symbols
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "id": s.id.0, "name": s.name, "kind": s.kind.as_str(),
                        "lines": [s.span.start_line, s.span.end_line], "signature": s.signature,
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::json!({ "file": file, "symbols": items, "count": items.len() })
            );
        }
        return Ok(());
    }

    if args.project || args.file.is_none() {
        print_project_outline(&index, args.depth, args.summary, color_on)?;
    } else if let Some(ref file) = args.file {
        let file_path = PathBuf::from(file);
        print_file_outline(&index, &file_path, color_on)?;
    }

    Ok(())
}

fn print_file_outline(
    index: &crate::model::project::ProjectIndex,
    file: &PathBuf,
    color_on: bool,
) -> anyhow::Result<()> {
    let symbols = index.symbols_in_file(file);

    if symbols.is_empty() {
        println!("No symbols found in {}", file.display());
        return Ok(());
    }

    println!(
        "{}",
        color::bold(
            &format!("{} ({} symbols)", file.display(), symbols.len()),
            color_on
        )
    );
    println!();

    let mut top_level: Vec<&Symbol> = symbols
        .iter()
        .filter(|s| s.parent.is_none())
        .copied()
        .collect();
    top_level.sort_by_key(|s| s.span.start_line);

    for (i, sym) in top_level.iter().enumerate() {
        let is_last = i == top_level.len() - 1;
        let prefix = if is_last { "└──" } else { "├──" };
        let child_prefix = if is_last { "    " } else { "│   " };

        print_symbol_line(prefix, sym, color_on);

        let children: Vec<&Symbol> = symbols
            .iter()
            .filter(|s| s.parent.as_ref() == Some(&sym.id))
            .copied()
            .collect();
        for (j, child) in children.iter().enumerate() {
            let is_child_last = j == children.len() - 1;
            let cp = if is_child_last {
                "└──"
            } else {
                "├──"
            };
            print!("{}", child_prefix);
            print_symbol_line(cp, child, color_on);
        }
    }

    Ok(())
}

fn print_project_outline(
    index: &crate::model::project::ProjectIndex,
    max_depth: u32,
    summary: bool,
    color_on: bool,
) -> anyhow::Result<()> {
    let stats = index.stats();
    println!(
        "{}",
        color::bold(
            &format!(
                "{} ({} files, {} symbols)",
                index.root.file_name().unwrap_or_default().to_string_lossy(),
                stats.total_files,
                stats.total_symbols,
            ),
            color_on
        ),
    );

    let mut dir_tree: BTreeMap<PathBuf, Vec<(&PathBuf, Vec<&Symbol>)>> = BTreeMap::new();
    for file in index.file_symbols.keys() {
        let dir = file.parent().unwrap_or(file).to_path_buf();
        let symbols = index.symbols_in_file(file);
        dir_tree.entry(dir).or_default().push((file, symbols));
    }

    for (dir, files) in &dir_tree {
        let depth = dir.components().count() as u32;
        if depth > max_depth {
            continue;
        }

        let total_syms: usize = files.iter().map(|(_, s)| s.len()).sum();
        println!(
            "├── {}",
            color::cyan(
                &format!(
                    "{}/ ({} files, {} symbols)",
                    dir.display(),
                    files.len(),
                    total_syms
                ),
                color_on
            ),
        );

        if !summary {
            for (file, symbols) in files {
                let top_names: Vec<String> = symbols
                    .iter()
                    .filter(|s| s.parent.is_none())
                    .filter(|s| {
                        matches!(
                            s.kind,
                            SymbolKind::Function
                                | SymbolKind::Struct
                                | SymbolKind::Class
                                | SymbolKind::Interface
                                | SymbolKind::Enum
                        )
                    })
                    .take(5)
                    .map(|s| s.name.clone())
                    .collect();

                let names_str = if top_names.is_empty() {
                    String::new()
                } else {
                    format!(" — {}", top_names.join(", "))
                };

                let file_name = file.file_name().unwrap_or_default().to_string_lossy();
                println!(
                    "│   ├── {}{} {}",
                    file_name,
                    names_str,
                    color::dim(&format!("[{} symbols]", symbols.len()), color_on),
                );
            }
        }
    }

    Ok(())
}

fn print_symbol_line(prefix: &str, sym: &Symbol, color_on: bool) {
    let sig_or_name = sym.signature.as_deref().unwrap_or(&sym.name);
    let display = if sig_or_name.len() > 80 {
        format!("{}...", &sig_or_name[..77])
    } else {
        sig_or_name.to_string()
    };

    println!(
        "{} {} {} {}",
        prefix,
        display,
        color::cyan(&format!("({})", sym.kind), color_on),
        color::dim(&format!("[L{}]", sym.span.start_line), color_on),
    );
}
