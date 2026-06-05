use crate::cli::RefsArgs;
use crate::output::color;
use crate::parser::traits::RefKind;
use std::borrow::Cow;
use std::path::PathBuf;

pub fn run(
    args: RefsArgs,
    root_override: Option<&str>,
    workspace_flag: bool,
    json: bool,
    color_on: bool,
) -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    let provider = crate::commands::IndexProvider::load(root_override, workspace_flag)?;

    // Use pre-computed identifier_index to find candidate files
    let candidate_keys = provider.identifier_files_for(&args.name);

    // Collect refs from pre-computed identifier tables
    let mut all_refs: Vec<(PathBuf, crate::parser::traits::IdentifierRef)> = Vec::new();
    for file_key in &candidate_keys {
        let scope_match = args
            .scope
            .as_ref()
            .is_none_or(|s| file_key.path.to_string_lossy().starts_with(s.as_str()));
        if !scope_match {
            continue;
        }

        let idents = provider.identifiers_in_file(file_key);
        for r in idents {
            if r.name == args.name {
                all_refs.push((file_key.path.clone(), r.clone()));
            }
        }
    }

    // Apply kind filter
    if let Some(ref kind_filter) = args.kind {
        let target_kind = match kind_filter.to_lowercase().as_str() {
            "call" => Some(RefKind::Call),
            "type" => Some(RefKind::TypeRef),
            "import" | "use" => Some(RefKind::Import),
            "field" => Some(RefKind::FieldAccess),
            "constructor" | "ctor" => Some(RefKind::Constructor),
            "def" | "definition" => Some(RefKind::Definition),
            _ => None,
        };

        if let Some(target) = target_kind {
            all_refs.retain(|(_, r)| r.kind == target);
        }
    }

    // Exclude definitions by default
    all_refs.retain(|(_, r)| r.kind != RefKind::Definition);

    if all_refs.is_empty() {
        println!("No references found for \"{}\"", args.name);
        return Ok(());
    }

    // Sort by file then line
    all_refs.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.line.cmp(&b.1.line)));

    // Count by kind
    let mut call_count = 0;
    let mut type_count = 0;
    let mut import_count = 0;
    let mut other_count = 0;
    for (_, r) in &all_refs {
        match r.kind {
            RefKind::Call | RefKind::Constructor => call_count += 1,
            RefKind::TypeRef => type_count += 1,
            RefKind::Import => import_count += 1,
            _ => other_count += 1,
        }
    }

    let total = all_refs.len();
    let all_files = provider.file_count();
    let scanned = candidate_keys.len();
    let narrowed = scanned < all_files;
    let breakdown = format_breakdown(call_count, type_count, import_count, other_count);

    // Apply offset then truncate
    if args.offset > 0 {
        let off = args.offset.min(all_refs.len());
        all_refs.drain(..off);
    }
    all_refs.truncate(args.limit);

    if json {
        let items: Vec<serde_json::Value> = all_refs
            .iter()
            .map(|(file, r)| {
                serde_json::json!({
                    "file": file.to_string_lossy(),
                    "line": r.line,
                    "context": r.context,
                    "kind": format!("{:?}", r.kind),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({ "name": args.name, "refs": items, "count": total })
        );
        return Ok(());
    }

    if narrowed {
        println!(
            "{} — {} refs ({}) [via identifier index, {} files]",
            color::bold(&args.name, color_on),
            total,
            breakdown,
            scanned,
        );
    } else {
        println!(
            "{} — {} refs ({})",
            color::bold(&args.name, color_on),
            total,
            breakdown
        );
    }

    for (file, r) in &all_refs {
        let kind_tag: Cow<'_, str> = match r.kind {
            RefKind::Call => color::yellow("[call]", color_on),
            RefKind::TypeRef => color::cyan("[type]", color_on),
            RefKind::Import => color::green("[import]", color_on),
            RefKind::FieldAccess => Cow::Borrowed("[field]"),
            RefKind::Constructor => color::yellow("[ctor]", color_on),
            RefKind::Definition => Cow::Borrowed("[def]"),
            RefKind::Unknown => Cow::Borrowed(""),
        };
        println!(
            "  {}:{:<6} {:<50} {}",
            file.display(),
            r.line,
            r.context,
            kind_tag,
        );
    }

    if std::env::var("SYMLENS_VERBOSE").is_ok() && !json {
        eprintln!(
            "[verbose] refs completed in {:?} (scanned {} files via index)",
            start.elapsed(),
            scanned
        );
    }

    Ok(())
}

fn format_breakdown(calls: usize, types: usize, imports: usize, other: usize) -> String {
    let mut parts = Vec::new();
    if calls > 0 {
        parts.push(format!("{} calls", calls));
    }
    if types > 0 {
        parts.push(format!("{} types", types));
    }
    if imports > 0 {
        parts.push(format!("{} imports", imports));
    }
    if other > 0 {
        parts.push(format!("{} other", other));
    }
    parts.join(", ")
}
