use crate::cli::SearchArgs;
use crate::index::storage;
use crate::model::symbol::{SymbolId, SymbolKind};
use crate::output::color;

pub fn run(
    args: SearchArgs,
    root_override: Option<&str>,
    json: bool,
    color_on: bool,
) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;

    let index = storage::load(&root)?
        .ok_or_else(|| anyhow::anyhow!("No index found. Run `codelens index` first."))?;

    let results = if let Ok(Some(engine)) = storage::open_search(&root) {
        let search_results = engine.search(&args.query, args.limit * 2)?;
        let mut syms: Vec<_> = search_results
            .iter()
            .filter_map(|r| {
                let id = SymbolId(r.symbol_id.clone());
                index.get(&id).map(|s| (s, r.score))
            })
            .collect();

        if let Some(ref kind_str) = args.kind {
            if let Some(kind) = SymbolKind::from_str(kind_str) {
                syms.retain(|(s, _)| s.kind == kind);
            }
        }

        if let Some(ref path_prefix) = args.path {
            syms.retain(|(s, _)| {
                s.file_path
                    .to_string_lossy()
                    .starts_with(path_prefix.as_str())
            });
        }

        syms.truncate(args.limit);
        syms
    } else {
        let mut results = index.search(&args.query, args.limit);

        if let Some(ref kind_str) = args.kind {
            if let Some(kind) = SymbolKind::from_str(kind_str) {
                results.retain(|s| s.kind == kind);
            }
        }

        if let Some(ref path_prefix) = args.path {
            results.retain(|s| {
                s.file_path
                    .to_string_lossy()
                    .starts_with(path_prefix.as_str())
            });
        }

        results.truncate(args.limit);
        results.into_iter().map(|s| (s, 0.0f32)).collect()
    };

    if results.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No symbols found matching \"{}\"", args.query);
        }
        return Ok(());
    }

    if json {
        println!("{}", crate::output::json::format_symbols(&results));
    } else {
        for (sym, _score) in &results {
            let kind_str = color::cyan(&format!("({})", sym.kind), color_on);
            println!(
                "{} {} {}",
                color::bold(&sym.id.0, color_on),
                kind_str,
                color::dim(&format!("[{}]", sym.span), color_on)
            );
            if let Some(ref sig) = sym.signature {
                println!("  {}", sig);
            }
            if let Some(ref doc) = sym.doc_comment {
                let first_line: &str = doc.lines().next().unwrap_or("");
                if !first_line.is_empty() {
                    println!("  {}", color::dim(&format!("/// {}", first_line), color_on));
                }
            }
            println!();
        }
        println!(
            "{}",
            color::dim(&format!("{} results", results.len()), color_on)
        );
    }
    Ok(())
}
