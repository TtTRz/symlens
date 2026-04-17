use crate::cli::RefsArgs;
use crate::model::project::FileKey;
use crate::output::color;
use crate::parser::registry::LanguageRegistry;
use crate::parser::traits::RefKind;
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;

pub fn run(
    args: RefsArgs,
    root_override: Option<&str>,
    workspace_flag: bool,
    json: bool,
    color_on: bool,
) -> anyhow::Result<()> {
    let provider = crate::commands::IndexProvider::load(root_override, workspace_flag)?;

    // Refs v3: narrow search scope using import_names
    // If we know which files import the target name, only search those + the defining file
    let candidate_keys: Vec<FileKey> = {
        let importing = provider.import_names_for(&args.name);
        if !importing.is_empty() {
            let mut keys: HashSet<FileKey> = importing.into_iter().collect();
            // Also include files that define a symbol with this name
            for sym in provider.symbols() {
                if sym.name == args.name {
                    keys.insert(FileKey::new(sym.id.root_id(), sym.file_path.clone()));
                }
            }
            keys.into_iter().collect()
        } else {
            // No import info — fall back to scanning all indexed files
            provider.file_keys()
        }
    };

    let scope = args.scope.clone();
    let name = args.name.clone();

    // Collect (root_id, rel_path) pairs for parallel resolution
    let scan_entries: Vec<(String, PathBuf)> = candidate_keys
        .into_iter()
        .map(|fk| (fk.root_id, fk.path))
        .collect();

    // Parallel file scanning (same pattern as indexer.rs — one registry per thread)
    let mut all_refs: Vec<(PathBuf, crate::parser::traits::IdentifierRef)> = scan_entries
        .par_iter()
        .filter(|(root_id, file_path)| {
            let full_path = provider.resolve_absolute(root_id, file_path);
            if !full_path.exists() {
                return false;
            }
            if let Some(ref s) = scope
                && !file_path.to_string_lossy().starts_with(s.as_str())
            {
                return false;
            }
            true
        })
        .flat_map_iter(|(root_id, file_path)| {
            let full_path = provider.resolve_absolute(root_id, file_path);
            let registry = LanguageRegistry::new();
            let mut results = Vec::new();
            if let Some(parser) = registry.parser_for(&full_path)
                && let Ok(source) = std::fs::read(&full_path)
                && let Ok(refs) = parser.find_identifiers(&source, &name)
            {
                for r in refs {
                    results.push((file_path.clone(), r));
                }
            }
            results
        })
        .collect();

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
    let scanned = scan_entries.len();
    let narrowed = scanned < all_files;
    let breakdown = format_breakdown(call_count, type_count, import_count, other_count);

    // Truncate
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
            "{} — {} refs ({}) [scanned {}/{} files via import tracking]",
            color::bold(&args.name, color_on),
            total,
            breakdown,
            scanned,
            all_files,
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
        let kind_tag = match r.kind {
            RefKind::Call => color::yellow("[call]", color_on),
            RefKind::TypeRef => color::cyan("[type]", color_on),
            RefKind::Import => color::green("[import]", color_on),
            RefKind::FieldAccess => "[field]".to_string(),
            RefKind::Constructor => color::yellow("[ctor]", color_on),
            RefKind::Definition => "[def]".to_string(),
            RefKind::Unknown => String::new(),
        };
        println!(
            "  {}:{:<6} {:<50} {}",
            file.display(),
            r.line,
            r.context,
            kind_tag,
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
