use crate::cli::DiffArgs;
use crate::model::symbol::SymbolKind;
use crate::parser::registry::LanguageRegistry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

pub fn run(args: DiffArgs, root_override: Option<&str>) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;
    let registry = LanguageRegistry::new();
    let mut changed_symbols: Vec<ChangedSymbol> = Vec::new();

    // ── Added files (A): all symbols are new ──────────────────────
    let added_files = git_diff_names(&root, &args.from, &args.to, "A")?;
    for file in &added_files {
        let file_path = PathBuf::from(file);
        let full_path = root.join(&file_path);
        if !registry.is_supported(&full_path) || !full_path.exists() {
            continue;
        }
        if let Some(parser) = registry.parser_for(&full_path) {
            if let Ok(source) = std::fs::read(&full_path) {
                if let Ok(symbols) = parser.extract_symbols(&source, &file_path) {
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
        }
    }

    // ── Modified files (M): only symbols whose lines overlap the diff ─
    let modified_files = git_diff_names(&root, &args.from, &args.to, "M")?;
    for file in &modified_files {
        let file_path = PathBuf::from(file);
        let full_path = root.join(&file_path);
        if !registry.is_supported(&full_path) || !full_path.exists() {
            continue;
        }

        let changed_ranges = git_diff_ranges(&root, &args.from, &args.to, file)?;
        if changed_ranges.is_empty() {
            continue;
        }

        if let Some(parser) = registry.parser_for(&full_path) {
            if let Ok(source) = std::fs::read(&full_path) {
                if let Ok(symbols) = parser.extract_symbols(&source, &file_path) {
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
        }
    }

    // ── Deleted files (D): mark entire file ──────────────────────
    let deleted_files = git_diff_names(&root, &args.from, &args.to, "D")?;
    for file in &deleted_files {
        let file_path = PathBuf::from(file);
        // Only track supported source files
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

    // ── Apply filters ────────────────────────────────────────────
    if let Some(ref kind_filter) = args.kind {
        if let Some(kind) = SymbolKind::from_str(kind_filter) {
            changed_symbols.retain(|s| s.kind == kind);
        }
    }

    // Deduplicate
    let mut seen = HashMap::new();
    changed_symbols.retain(|s| {
        let key = format!("{}::{}", s.file, s.name);
        seen.insert(key, ()).is_none()
    });

    if changed_symbols.is_empty() {
        println!("No symbol changes between {} and {}", args.from, args.to);
        return Ok(());
    }

    // ── Output ───────────────────────────────────────────────────
    let mut by_file: HashMap<String, Vec<&ChangedSymbol>> = HashMap::new();
    for sym in &changed_symbols {
        by_file.entry(sym.file.clone()).or_default().push(sym);
    }

    let mut files: Vec<_> = by_file.keys().cloned().collect();
    files.sort();

    let total_files = files.len();
    let total_symbols = changed_symbols.len();
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

    println!(
        "Changed symbols: {} → {} ({} symbols in {} files — +{} ~{} -{})",
        args.from, args.to, total_symbols, total_files, added_count, modified_count, deleted_count,
    );
    println!();

    for file in &files {
        let syms = by_file.get(file).unwrap();
        println!("{} ({} changes)", file, syms.len());
        for sym in syms {
            let change_marker = match sym.change_kind {
                ChangeKind::Added => "+",
                ChangeKind::Modified => "~",
                ChangeKind::Deleted => "-",
            };

            let sig = sym.signature.as_deref().unwrap_or(&sym.name);
            let sig_display = if sig.len() > 70 {
                format!("{}...", &sig[..67])
            } else {
                sig.to_string()
            };

            if sym.span_start > 0 {
                println!(
                    "  {} {} ({}) [L{}-{}]",
                    change_marker, sig_display, sym.kind, sym.span_start, sym.span_end,
                );
            } else {
                println!("  {} {}", change_marker, sig_display);
            }
        }
        println!();
    }

    if total_symbols > 0 {
        println!(
            "Tip: run `codelens graph impact \"<symbol>\"` to check blast radius of modified symbols."
        );
    }

    Ok(())
}

#[derive(Debug)]
struct ChangedSymbol {
    file: String,
    name: String,
    kind: SymbolKind,
    span_start: u32,
    span_end: u32,
    change_kind: ChangeKind,
    signature: Option<String>,
}

#[derive(Debug)]
enum ChangeKind {
    Added,
    Modified,
    Deleted,
}

/// Get file names from git diff with a specific filter (A/M/D/etc).
fn git_diff_names(
    root: &PathBuf,
    from: &str,
    to: &str,
    filter: &str,
) -> anyhow::Result<Vec<String>> {
    let output = Command::new("git")
        .args([
            "-C",
            &root.to_string_lossy(),
            "diff",
            "--name-only",
            &format!("--diff-filter={}", filter),
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
        .map(|l| l.to_string())
        .collect())
}

/// Get changed line ranges for a specific file from git diff.
fn git_diff_ranges(
    root: &PathBuf,
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
        if line.starts_with("@@") {
            if let Some(plus_part) = line.split('+').nth(1) {
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
    }
    ranges
}

fn ranges_overlap(a_start: u32, a_end: u32, b_start: u32, b_end: u32) -> bool {
    a_start <= b_end && b_start <= a_end
}
