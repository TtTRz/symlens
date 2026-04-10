use crate::cli::RefsArgs;
use crate::index::storage;
use crate::parser::registry::LanguageRegistry;
use crate::parser::traits::RefKind;
use std::collections::HashSet;
use std::path::PathBuf;

pub fn run(args: RefsArgs, root_override: Option<&str>) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;

    let index = storage::load(&root)?
        .ok_or_else(|| anyhow::anyhow!("No index found. Run `codelens index` first."))?;

    let registry = LanguageRegistry::new();

    // Refs v3: narrow search scope using import_names
    // If we know which files import the target name, only search those + the defining file
    let candidate_files: Vec<PathBuf> =
        if let Some(importing_files) = index.import_names.get(&args.name) {
            // Files that import this name + files where it's defined
            let mut files: HashSet<PathBuf> = importing_files.iter().cloned().collect();
            // Also include files that define a symbol with this name
            for sym in index.symbols.values() {
                if sym.name == args.name {
                    files.insert(sym.file_path.clone());
                }
            }
            files.into_iter().collect()
        } else {
            // No import info — fall back to scanning all indexed files
            index.file_symbols.keys().cloned().collect()
        };

    let mut all_refs = Vec::new();

    for file_path in &candidate_files {
        let full_path = root.join(file_path);
        if !full_path.exists() {
            continue;
        }

        // Apply scope filter
        if let Some(ref scope) = args.scope {
            if !file_path.to_string_lossy().starts_with(scope.as_str()) {
                continue;
            }
        }

        if let Some(parser) = registry.parser_for(&full_path) {
            if let Ok(source) = std::fs::read(&full_path) {
                if let Ok(refs) = parser.find_identifiers(&source, &args.name) {
                    for r in refs {
                        all_refs.push((file_path.clone(), r));
                    }
                }
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
    let all_files = index.file_symbols.len();
    let scanned = candidate_files.len();
    let narrowed = scanned < all_files;
    let breakdown = format_breakdown(call_count, type_count, import_count, other_count);

    // Truncate
    all_refs.truncate(args.limit);

    if narrowed {
        println!(
            "{} — {} refs ({}) [scanned {}/{} files via import tracking]",
            args.name, total, breakdown, scanned, all_files,
        );
    } else {
        println!("{} — {} refs ({})", args.name, total, breakdown);
    }

    for (file, r) in &all_refs {
        let kind_tag = match r.kind {
            RefKind::Call => "[call]",
            RefKind::TypeRef => "[type]",
            RefKind::Import => "[import]",
            RefKind::FieldAccess => "[field]",
            RefKind::Constructor => "[ctor]",
            RefKind::Definition => "[def]",
            RefKind::Unknown => "",
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
