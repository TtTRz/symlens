use crate::cli::SymbolArgs;
use crate::index::storage;
use crate::model::symbol::SymbolId;

pub fn run(args: SymbolArgs) -> anyhow::Result<()> {
    let root = {
        let cwd = std::env::current_dir()?;
        storage::find_project_root(&cwd).unwrap_or(cwd)
    };

    let index = storage::load(&root)?
        .ok_or_else(|| anyhow::anyhow!("No index found. Run `codelens index` first."))?;

    let id = SymbolId(args.symbol_id.clone());
    let symbol = index
        .get(&id)
        .ok_or_else(|| anyhow::anyhow!("Symbol not found: {}", args.symbol_id))?;

    // Get source if requested
    let source_text = if args.source {
        let source_file = root.join(&symbol.file_path);
        if source_file.exists() {
            let content = std::fs::read_to_string(&source_file)?;
            let lines: Vec<&str> = content.lines().collect();
            let start = (symbol.span.start_line as usize).saturating_sub(1);
            let end = (symbol.span.end_line as usize).min(lines.len());
            Some(lines[start..end].join("\n"))
        } else {
            None
        }
    } else {
        None
    };

    if args.json {
        println!(
            "{}",
            crate::output::json::format_symbol(symbol, source_text.as_deref())
        );
        return Ok(());
    }

    // Compact output
    println!(
        "{} ({}) [{}: {}]",
        symbol.qualified_name,
        symbol.kind,
        symbol.file_path.display(),
        symbol.span,
    );

    if let Some(ref sig) = symbol.signature {
        println!("  {}", sig);
    }

    if let Some(ref doc) = symbol.doc_comment {
        for line in doc.lines() {
            println!("  /// {}", line);
        }
    }

    if let Some(ref parent) = symbol.parent {
        println!("  Parent: {}", parent);
    }

    if let Some(ref src) = source_text {
        println!("───────────────────────────────────────");
        let start = symbol.span.start_line as usize;
        for (i, line) in src.lines().enumerate() {
            println!("{:>4} {}", start + i, line);
        }
    }

    Ok(())
}
