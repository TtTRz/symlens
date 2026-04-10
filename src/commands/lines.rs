use crate::cli::LinesArgs;
use crate::index::storage;

pub fn run(args: LinesArgs) -> anyhow::Result<()> {
    let root = {
        let cwd = std::env::current_dir()?;
        storage::find_project_root(&cwd).unwrap_or(cwd)
    };

    let full_path = root.join(&args.file);
    if !full_path.exists() {
        anyhow::bail!("File not found: {}", args.file);
    }

    let content = std::fs::read_to_string(&full_path)?;
    let lines: Vec<&str> = content.lines().collect();

    let start = (args.start as usize).saturating_sub(1);
    let end = (args.end as usize).min(lines.len());

    if start >= lines.len() {
        anyhow::bail!("Start line {} exceeds file length {}", args.start, lines.len());
    }

    // Cap at 500 lines
    let max_lines = 500;
    let actual_end = end.min(start + max_lines);

    for (i, line) in lines[start..actual_end].iter().enumerate() {
        println!("{:>4} {}", start + i + 1, line);
    }

    if actual_end < end {
        println!("... truncated ({} more lines)", end - actual_end);
    }

    Ok(())
}
