use crate::cli::BlameArgs;
use crate::index::storage;
use std::process::Command;

pub fn run(args: BlameArgs, root_override: Option<&str>, json: bool) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;

    let index = storage::load(&root)?
        .ok_or_else(|| anyhow::anyhow!("No index found. Run `codelens index` first."))?;

    // Find the symbol (try exact match first, then partial)
    let symbol = index
        .symbols
        .values()
        .find(|s| s.id.0 == args.name || s.qualified_name == args.name || s.name == args.name)
        .ok_or_else(|| anyhow::anyhow!("Symbol not found: {}", args.name))?;

    let file_path = root.join(&symbol.file_path);
    if !file_path.exists() {
        anyhow::bail!("File not found: {}", symbol.file_path.display());
    }

    let start = symbol.span.start_line;
    let end = symbol.span.end_line;

    // Run git blame on the symbol's line range
    let output = Command::new("git")
        .args([
            "-C",
            &root.to_string_lossy(),
            "blame",
            "--porcelain",
            &format!("-L{},{}", start, end),
            &symbol.file_path.to_string_lossy(),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git blame failed: {}", stderr.trim());
    }

    let blame_output = String::from_utf8_lossy(&output.stdout);
    let blame_info = parse_porcelain_blame(&blame_output);

    if json {
        let commits: Vec<serde_json::Value> = blame_info
            .iter()
            .map(|b| {
                serde_json::json!({
                    "commit": b.commit, "author": b.author, "date": b.date,
                    "summary": b.summary, "line": b.line, "content": b.content,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "symbol": symbol.qualified_name,
                "kind": symbol.kind.as_str(),
                "file": symbol.file_path.to_string_lossy(),
                "lines": [start, end],
                "blame": commits,
            })
        );
        return Ok(());
    }

    // Print header
    println!(
        "{} ({}) [{}: L{}-{}]",
        symbol.qualified_name,
        symbol.kind,
        symbol.file_path.display(),
        start,
        end,
    );
    println!();

    if blame_info.is_empty() {
        println!("No git blame information available.");
        return Ok(());
    }

    // Collect unique commits
    let mut seen_commits = std::collections::HashSet::new();
    let mut unique_commits = Vec::new();
    for info in &blame_info {
        if seen_commits.insert(info.commit.clone()) {
            unique_commits.push(info);
        }
    }

    // Sort by date (newest first)
    let mut sorted_commits = unique_commits;
    sorted_commits.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    println!("Authors ({} commits):", sorted_commits.len());
    for info in &sorted_commits {
        println!(
            "  {} {} [{}]  {}",
            &info.commit[..8.min(info.commit.len())],
            info.author,
            info.date,
            info.summary,
        );
    }

    // Print per-line blame
    println!();
    println!("Lines:");
    for info in &blame_info {
        println!(
            "  {:>4} {} {:<20} {}",
            info.line,
            &info.commit[..8.min(info.commit.len())],
            info.author,
            info.content,
        );
    }

    Ok(())
}

struct BlameInfo {
    commit: String,
    author: String,
    date: String,
    summary: String,
    timestamp: u64,
    line: u32,
    content: String,
}

fn parse_porcelain_blame(output: &str) -> Vec<BlameInfo> {
    let mut results = Vec::new();
    let mut current_commit = String::new();
    let mut current_author = String::new();
    let mut current_date = String::new();
    let mut current_summary = String::new();
    let mut current_timestamp: u64 = 0;
    let mut current_line: u32 = 0;
    let mut commit_data: std::collections::HashMap<String, (String, String, String, u64)> =
        std::collections::HashMap::new();

    for line in output.lines() {
        if line.starts_with('\t') {
            // Content line
            results.push(BlameInfo {
                commit: current_commit.clone(),
                author: current_author.clone(),
                date: current_date.clone(),
                summary: current_summary.clone(),
                timestamp: current_timestamp,
                line: current_line,
                content: line[1..].to_string(),
            });
        } else if let Some(rest) = line.strip_prefix("author ") {
            current_author = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("author-time ") {
            current_timestamp = rest.parse().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("committer-time ") {
            // Format timestamp as date
            let ts: i64 = rest.parse().unwrap_or(0);
            current_date = format_timestamp(ts);
        } else if let Some(rest) = line.strip_prefix("summary ") {
            current_summary = rest.to_string();
            // Cache commit data
            commit_data.insert(
                current_commit.clone(),
                (
                    current_author.clone(),
                    current_date.clone(),
                    current_summary.clone(),
                    current_timestamp,
                ),
            );
        } else {
            // First line of a blame entry: <sha1> <orig_line> <final_line> [<num_lines>]
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[0].len() >= 20 {
                current_commit = parts[0].to_string();
                current_line = parts[2].parse().unwrap_or(0);
                // Restore cached commit data if we've seen this commit before
                if let Some((author, date, summary, ts)) = commit_data.get(&current_commit) {
                    current_author = author.clone();
                    current_date = date.clone();
                    current_summary = summary.clone();
                    current_timestamp = *ts;
                }
            }
        }
    }

    results
}

fn format_timestamp(ts: i64) -> String {
    if ts == 0 {
        return "unknown".to_string();
    }
    // Simple date formatting without chrono dependency
    let days_since_epoch = ts / 86400;
    let mut y = 1970i64;
    let mut remaining = days_since_epoch;

    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }

    let months = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut m = 0;
    for (i, &days) in months.iter().enumerate() {
        if remaining < days {
            m = i + 1;
            break;
        }
        remaining -= days;
    }
    let d = remaining + 1;

    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
