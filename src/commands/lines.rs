use crate::cli::LinesArgs;
use crate::output::color;

pub fn run(
    args: LinesArgs,
    root_override: Option<&str>,
    workspace_flag: bool,
    color_on: bool,
) -> anyhow::Result<()> {
    let provider = crate::commands::IndexProvider::load(root_override, workspace_flag)?;

    // Resolve the file: in single-root mode just join; in workspace mode,
    // search through file_keys to find the matching relative path.
    let full_path = if let Some(root) = provider.single_root() {
        root.join(&args.file)
    } else {
        // Workspace mode: find which root contains this file
        let rel = std::path::Path::new(&args.file);
        let keys = provider.file_keys();
        let matched = keys.iter().find(|fk| fk.path == rel);
        match matched {
            Some(fk) => provider.resolve_absolute(&fk.root_id, &fk.path),
            None => {
                // File not in index — try each root as fallback
                anyhow::bail!(
                    "File \"{}\" not found in workspace index. Run `symlens index` first.",
                    args.file
                );
            }
        }
    };

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
