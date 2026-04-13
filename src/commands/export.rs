use crate::cli::ExportArgs;
use crate::index::storage;

pub fn run(args: ExportArgs, root_override: Option<&str>) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;

    let index = storage::load(&root)?
        .ok_or_else(|| anyhow::anyhow!("No index found. Run `codelens index` first."))?;

    match args.format.as_str() {
        "json" => export_json(&index, args.output.as_deref()),
        _ => anyhow::bail!(
            "Unsupported export format: '{}'. Supported: json",
            args.format
        ),
    }
}

fn export_json(
    index: &crate::model::project::ProjectIndex,
    output_path: Option<&str>,
) -> anyhow::Result<()> {
    let stats = index.stats();

    let symbols: Vec<serde_json::Value> = index
        .symbols
        .values()
        .map(|s| {
            let mut obj = serde_json::json!({
                "id": s.id.0,
                "name": s.name,
                "qualified_name": s.qualified_name,
                "kind": s.kind.as_str(),
                "file": s.file_path.to_string_lossy(),
                "start_line": s.span.start_line,
                "end_line": s.span.end_line,
                "visibility": format!("{:?}", s.visibility),
            });
            if let Some(ref sig) = s.signature {
                obj["signature"] = serde_json::Value::String(sig.clone());
            }
            if let Some(ref doc) = s.doc_comment {
                obj["doc"] = serde_json::Value::String(doc.clone());
            }
            if let Some(ref parent) = s.parent {
                obj["parent"] = serde_json::Value::String(parent.0.clone());
            }
            obj
        })
        .collect();

    let call_edges: Vec<serde_json::Value> = if let Some(ref cg) = index.call_graph {
        cg.all_edges()
            .iter()
            .map(|&(from, to)| serde_json::json!({ "caller": &cg.nodes[from], "callee": &cg.nodes[to] }))
            .collect()
    } else {
        vec![]
    };

    let files: Vec<serde_json::Value> = index
        .file_symbols
        .iter()
        .map(|(file, ids)| {
            serde_json::json!({
                "file": file.to_string_lossy(),
                "symbols": ids.len(),
                "mtime": index.file_mtimes.get(file).unwrap_or(&0),
            })
        })
        .collect();

    let export = serde_json::json!({
        "version": index.version,
        "root": index.root.to_string_lossy(),
        "stats": {
            "files": stats.total_files,
            "symbols": stats.total_symbols,
            "by_language": stats.by_language,
            "by_kind": stats.by_kind,
        },
        "symbols": symbols,
        "call_edges": call_edges,
        "files": files,
    });

    let json_str = serde_json::to_string_pretty(&export)?;

    if let Some(path) = output_path {
        std::fs::write(path, &json_str)?;
        eprintln!(
            "Exported {} symbols + {} call edges to {}",
            symbols.len(),
            call_edges.len(),
            path,
        );
    } else {
        println!("{}", json_str);
    }

    Ok(())
}
