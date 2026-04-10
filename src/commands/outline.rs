use crate::cli::OutlineArgs;
use crate::index::storage;
use crate::model::symbol::{Symbol, SymbolKind};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub fn run(args: OutlineArgs, root_override: Option<&str>) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;

    let index = storage::load(&root)?
        .ok_or_else(|| anyhow::anyhow!("No index found. Run `codelens index` first."))?;

    if args.project || args.file.is_none() {
        print_project_outline(&index, args.depth, args.summary)?;
    } else if let Some(ref file) = args.file {
        let file_path = PathBuf::from(file);
        print_file_outline(&index, &file_path)?;
    }

    Ok(())
}

fn print_file_outline(
    index: &crate::model::project::ProjectIndex,
    file: &PathBuf,
) -> anyhow::Result<()> {
    let symbols = index.symbols_in_file(file);

    if symbols.is_empty() {
        println!("No symbols found in {}", file.display());
        return Ok(());
    }

    println!("{} ({} symbols)", file.display(), symbols.len());

    // Group: top-level first, then children
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

        print_symbol_line(prefix, sym);

        // Print children
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
            print_symbol_line(cp, child);
        }
    }

    Ok(())
}

fn print_project_outline(
    index: &crate::model::project::ProjectIndex,
    max_depth: u32,
    summary: bool,
) -> anyhow::Result<()> {
    let stats = index.stats();
    println!(
        "{} ({} files, {} symbols)",
        index.root.file_name().unwrap_or_default().to_string_lossy(),
        stats.total_files,
        stats.total_symbols,
    );

    // Group files by directory
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
            "├── {}/ ({} files, {} symbols)",
            dir.display(),
            files.len(),
            total_syms,
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
                    "│   ├── {}{} [{} symbols]",
                    file_name,
                    names_str,
                    symbols.len(),
                );
            }
        }
    }

    Ok(())
}

fn print_symbol_line(prefix: &str, sym: &Symbol) {
    let sig_or_name = sym.signature.as_deref().unwrap_or(&sym.name);

    // Truncate long signatures
    let display = if sig_or_name.len() > 80 {
        format!("{}...", &sig_or_name[..77])
    } else {
        sig_or_name.to_string()
    };

    println!(
        "{} {} ({}) [L{}]",
        prefix, display, sym.kind, sym.span.start_line,
    );
}
