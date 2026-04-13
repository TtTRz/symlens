use crate::cli::LinesArgs;
use crate::output::color;

pub fn run(args: LinesArgs, root_override: Option<&str>, color_on: bool) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;

    let full_path = root.join(&args.file);
    if !full_path.exists() {
        anyhow::bail!("File not found: {}", args.file);
    }

    let content = std::fs::read_to_string(&full_path)?;
    let lines: Vec<&str> = content.lines().collect();

    let start = (args.start as usize).saturating_sub(1);
    let end = (args.end as usize).min(lines.len());

    if start >= lines.len() {
        anyhow::bail!(
            "Start line {} exceeds file length {}",
            args.start,
            lines.len()
        );
    }

    let max_lines = 500;
    let actual_end = end.min(start + max_lines);

    for (i, line) in lines[start..actual_end].iter().enumerate() {
        println!(
            "{} {}",
            color::dim(&format!("{:>4}", start + i + 1), color_on),
            line
        );
    }

    if actual_end < end {
        println!(
            "{}",
            color::dim(
                &format!("... truncated ({} more lines)", end - actual_end),
                color_on
            )
        );
    }

    Ok(())
}
