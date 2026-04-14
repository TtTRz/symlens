use crate::cli::DiffArgs;
use crate::model::symbol::SymbolKind;
use crate::output::color;
use crate::parser::registry::LanguageRegistry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// Result of a diff analysis between two git refs.
pub struct DiffResult {
    pub changes: Vec<ChangedSymbol>,
    pub added_count: usize,
    pub modified_count: usize,
    pub deleted_count: usize,
}

#[derive(Debug)]
pub struct ChangedSymbol {
    pub file: String,
    pub name: String,
    pub kind: SymbolKind,
    pub span_start: u32,
    pub span_end: u32,
    pub change_kind: ChangeKind,
    pub signature: Option<String>,
}

#[derive(Debug)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
}

/// Collect changed symbols between two git refs.
/// This is the core logic shared by CLI and MCP.
pub fn collect_changes(
    root: &std::path::Path,
    from: &str,
    to: &str,
    kind_filter: Option<&str>,
) -> anyhow::Result<DiffResult> {
    let registry = LanguageRegistry::new();
    let mut changed_symbols: Vec<ChangedSymbol> = Vec::new();

    // Single git call to get all changed files with status
    let name_status = git_diff_name_status(root, from, to)?;
    let added_files: Vec<_> = name_status
        .iter()
        .filter(|(s, _)| *s == 'A')
        .map(|(_, f)| f.clone())
        .collect();
    let modified_files: Vec<_> = name_status
        .iter()
        .filter(|(s, _)| *s == 'M')
        .map(|(_, f)| f.clone())
        .collect();
    let deleted_files: Vec<_> = name_status
        .iter()
        .filter(|(s, _)| *s == 'D')
        .map(|(_, f)| f.clone())
        .collect();

    // Added files (A): all symbols are new
    for file in &added_files {
        let file_path = PathBuf::from(file);
        let full_path = root.join(&file_path);
        if !registry.is_supported(&full_path) || !full_path.exists() {
            continue;
        }
        if let Some(parser) = registry.parser_for(&full_path)
            && let Ok(source) = std::fs::read(&full_path)
            && let Ok(symbols) = parser.extract_symbols(&source, &file_path)
        {
            for sym in &symbols {
                changed_symbols.push(ChangedSymbol {
                    file: file.clone(),
                    name: sym.qualified_name.clone(),
                    kind: sym.kind,
                    span_start: sym.span.start_line,
                    span_end: sym.span.end_line,
                    change_kind: ChangeKind::Added,
                    signature: sym.signature.clone(),
                });
            }
        }
    }

    // Modified files (M): only symbols whose lines overlap the diff
    for file in &modified_files {
        let file_path = PathBuf::from(file);
        let full_path = root.join(&file_path);
        if !registry.is_supported(&full_path) || !full_path.exists() {
            continue;
        }

        let changed_ranges = git_diff_ranges(root, from, to, file)?;
        if changed_ranges.is_empty() {
            continue;
        }

        if let Some(parser) = registry.parser_for(&full_path)
            && let Ok(source) = std::fs::read(&full_path)
            && let Ok(symbols) = parser.extract_symbols(&source, &file_path)
        {
            for sym in &symbols {
                for &(start, end) in &changed_ranges {
                    if ranges_overlap(start, end, sym.span.start_line, sym.span.end_line) {
                        changed_symbols.push(ChangedSymbol {
                            file: file.clone(),
                            name: sym.qualified_name.clone(),
                            kind: sym.kind,
                            span_start: sym.span.start_line,
                            span_end: sym.span.end_line,
                            change_kind: ChangeKind::Modified,
                            signature: sym.signature.clone(),
                        });
                        break;
                    }
                }
            }
        }
    }

    // Deleted files (D): mark entire file
    for file in &deleted_files {
        let file_path = PathBuf::from(file);
        if !registry.is_supported(&file_path) {
            continue;
        }
        changed_symbols.push(ChangedSymbol {
            file: file.clone(),
            name: "[entire file deleted]".to_string(),
            kind: SymbolKind::Module,
            span_start: 0,
            span_end: 0,
            change_kind: ChangeKind::Deleted,
            signature: None,
        });
    }

    // Apply filters
    if let Some(kf) = kind_filter
        && let Some(kind) = SymbolKind::from_str(kf)
    {
        changed_symbols.retain(|s| s.kind == kind);
    }

    // Deduplicate
    let mut seen = HashMap::new();
    changed_symbols.retain(|s| {
        let key = format!("{}::{}", s.file, s.name);
        seen.insert(key, ()).is_none()
    });

    let added_count = changed_symbols
        .iter()
        .filter(|s| matches!(s.change_kind, ChangeKind::Added))
        .count();
    let modified_count = changed_symbols
        .iter()
        .filter(|s| matches!(s.change_kind, ChangeKind::Modified))
        .count();
    let deleted_count = changed_symbols
        .iter()
        .filter(|s| matches!(s.change_kind, ChangeKind::Deleted))
        .count();

    Ok(DiffResult {
        changes: changed_symbols,
        added_count,
        modified_count,
        deleted_count,
    })
}

pub fn run(
    args: DiffArgs,
    root_override: Option<&str>,
    json: bool,
    color_on: bool,
) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;

    let result = collect_changes(&root, &args.from, &args.to, args.kind.as_deref())?;

    if result.changes.is_empty() {
        if json {
            println!(
                "{}",
                serde_json::json!({ "changes": [], "from": args.from, "to": args.to })
            );
        } else {
            println!("No symbol changes between {} and {}", args.from, args.to);
        }
        return Ok(());
    }

    let total_symbols = result.changes.len();

    if json {
        let items: Vec<serde_json::Value> = result.changes.iter().map(|s| {
            serde_json::json!({
                "file": s.file, "name": s.name, "kind": s.kind.as_str(),
                "change": match s.change_kind { ChangeKind::Added => "added", ChangeKind::Modified => "modified", ChangeKind::Deleted => "deleted" },
                "lines": [s.span_start, s.span_end], "signature": s.signature,
            })
        }).collect();
        println!(
            "{}",
            serde_json::json!({
                "from": args.from, "to": args.to,
                "changes": items, "total": total_symbols,
                "added": result.added_count, "modified": result.modified_count, "deleted": result.deleted_count,
            })
        );
        return Ok(());
    }

    // Group by file for text output
    let mut by_file: HashMap<String, Vec<&ChangedSymbol>> = HashMap::new();
    for sym in &result.changes {
        by_file.entry(sym.file.clone()).or_default().push(sym);
    }

    let mut files: Vec<_> = by_file.keys().cloned().collect();
    files.sort();

    let total_files = files.len();

    println!(
        "Changed symbols: {} → {} ({} symbols in {} files — {}+ {}~ {}-)",
        args.from,
        args.to,
        total_symbols,
        total_files,
        color::green(&format!("{}", result.added_count), color_on),
        color::yellow(&format!("{}", result.modified_count), color_on),
        color::red(&format!("{}", result.deleted_count), color_on),
    );
    println!();

    for file in &files {
        let syms = by_file.get(file).unwrap();
        println!("{} ({} changes)", color::bold(file, color_on), syms.len());
        for sym in syms {
            let (marker, marker_fn): (&str, fn(&str, bool) -> String) = match sym.change_kind {
                ChangeKind::Added => ("+", color::green),
                ChangeKind::Modified => ("~", color::yellow),
                ChangeKind::Deleted => ("-", color::red),
            };

            let sig = sym.signature.as_deref().unwrap_or(&sym.name);
            let sig_display = if sig.len() > 70 {
                format!("{}...", &sig[..67])
            } else {
                sig.to_string()
            };

            if sym.span_start > 0 {
                println!(
                    "  {} {} {} {}",
                    marker_fn(marker, color_on),
                    sig_display,
                    color::cyan(&format!("({})", sym.kind), color_on),
                    color::dim(&format!("[L{}-{}]", sym.span_start, sym.span_end), color_on),
                );
            } else {
                println!("  {} {}", marker_fn(marker, color_on), sig_display);
            }
        }
        println!();
    }

    if total_symbols > 0 {
        println!(
            "{}",
            color::dim(
                "Tip: run `symlens graph impact \"<symbol>\"` to check blast radius of modified symbols.",
                color_on
            ),
        );
    }

    Ok(())
}

/// Get file names with status from a single `git diff --name-status` call.
/// Returns (status_char, filename) pairs, e.g. ('A', "src/foo.rs").
fn git_diff_name_status(
    root: &std::path::Path,
    from: &str,
    to: &str,
) -> anyhow::Result<Vec<(char, String)>> {
    let output = Command::new("git")
        .args([
            "-C",
            &root.to_string_lossy(),
            "diff",
            "--name-status",
            from,
            to,
        ])
        .output()?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| {
            let mut parts = l.splitn(2, '\t');
            let status = parts.next()?.chars().next()?;
            let file = parts.next()?.to_string();
            Some((status, file))
        })
        .collect())
}

/// Get changed line ranges for a specific file from git diff.
fn git_diff_ranges(
    root: &std::path::Path,
    from: &str,
    to: &str,
    file: &str,
) -> anyhow::Result<Vec<(u32, u32)>> {
    let output = Command::new("git")
        .args([
            "-C",
            &root.to_string_lossy(),
            "diff",
            "-U0",
            from,
            to,
            "--",
            file,
        ])
        .output()?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    Ok(parse_diff_ranges(&String::from_utf8_lossy(&output.stdout)))
}

fn parse_diff_ranges(diff: &str) -> Vec<(u32, u32)> {
    let mut ranges = Vec::new();
    for line in diff.lines() {
        if line.starts_with("@@")
            && let Some(plus_part) = line.split('+').nth(1)
        {
            let nums = plus_part.split_whitespace().next().unwrap_or("");
            let parts: Vec<&str> = nums.split(',').collect();
            let start: u32 = parts[0].parse().unwrap_or(0);
            let count: u32 = if parts.len() > 1 {
                parts[1].parse().unwrap_or(1)
            } else {
                1
            };
            if start > 0 {
                ranges.push((start, start + count.saturating_sub(1)));
            }
        }
    }
    ranges
}

fn ranges_overlap(a_start: u32, a_end: u32, b_start: u32, b_end: u32) -> bool {
    a_start <= b_end && b_start <= a_end
}
