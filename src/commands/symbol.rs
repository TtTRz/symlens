use crate::cli::SymbolArgs;
use crate::index::storage;
use crate::model::symbol::SymbolId;
use crate::output::color;

pub fn run(
    args: SymbolArgs,
    root_override: Option<&str>,
    json: bool,
    color_on: bool,
) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;

    let index = storage::load(&root)?
        .ok_or_else(|| anyhow::anyhow!("No index found. Run `symlens index` first."))?;

    let id = SymbolId(args.symbol_id.clone());
    let symbol = index
        .get(&id)
        .ok_or_else(|| anyhow::anyhow!("Symbol not found: {}", args.symbol_id))?;

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

    if json {
        println!(
            "{}",
            crate::output::json::format_symbol(symbol, source_text.as_deref())
        );
        return Ok(());
    }

    println!(
        "{} {} {}",
        color::bold(&symbol.qualified_name, color_on),
        color::cyan(&format!("({})", symbol.kind), color_on),
        color::dim(
            &format!("[{}: {}]", symbol.file_path.display(), symbol.span),
            color_on
        ),
    );

    if let Some(ref sig) = symbol.signature {
        println!("  {}", sig);
    }

    if let Some(ref doc) = symbol.doc_comment {
        for line in doc.lines() {
            println!("  {}", color::dim(&format!("/// {}", line), color_on));
        }
    }

    if let Some(ref parent) = symbol.parent {
        println!("  Parent: {}", parent);
    }

    if let Some(ref src) = source_text {
        println!(
            "{}",
            color::dim("───────────────────────────────────────", color_on)
        );
        let start = symbol.span.start_line as usize;
        for (i, line) in src.lines().enumerate() {
            println!(
                "{} {}",
                color::dim(&format!("{:>4}", start + i), color_on),
                line
            );
        }
    }

    Ok(())
}
