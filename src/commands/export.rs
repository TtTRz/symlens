use crate::cli::ExportArgs;
use crate::index::storage;

pub fn run(args: ExportArgs, root_override: Option<&str>) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;

    let index = storage::load(&root)?
        .ok_or_else(|| anyhow::anyhow!("No index found. Run `symlens index` first."))?;

    match args.format.as_str() {
        "json" => export_json(&index, args.output.as_deref()),
        #[cfg(feature = "native")]
        "sqlite" => export_sqlite(&index, args.output.as_deref(), &root),
        _ => {
            let supported = if cfg!(feature = "native") {
                "json, sqlite"
            } else {
                "json"
            };
            anyhow::bail!(
                "Unsupported export format: '{}'. Supported: {}",
                args.format,
                supported
            )
        }
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

#[cfg(feature = "native")]
fn export_sqlite(
    index: &crate::model::project::ProjectIndex,
    output_path: Option<&str>,
    root: &std::path::Path,
) -> anyhow::Result<()> {
    use rusqlite::Connection;

    let db_path = match output_path {
        Some(p) => std::path::PathBuf::from(p),
        None => {
            let hash = blake3::hash(root.to_string_lossy().as_bytes()).to_hex()[..16].to_string();
            let cache_dir = dirs_cache_dir()?;
            std::fs::create_dir_all(&cache_dir)?;
            cache_dir.join(format!("symlens-{}.db", hash))
        }
    };

    // Remove existing file to avoid stale data
    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
    }

    let conn = Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;

    // Create tables
    conn.execute_batch(
        "CREATE TABLE symbols (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            qualified_name TEXT NOT NULL,
            kind TEXT NOT NULL,
            file TEXT NOT NULL,
            start_line INTEGER NOT NULL,
            end_line INTEGER NOT NULL,
            visibility TEXT,
            signature TEXT,
            doc TEXT,
            parent TEXT
        );
        CREATE TABLE call_edges (
            caller TEXT NOT NULL,
            callee TEXT NOT NULL
        );
        CREATE TABLE files (
            path TEXT PRIMARY KEY,
            symbol_count INTEGER NOT NULL,
            mtime INTEGER NOT NULL
        );
        CREATE TABLE metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )?;

    // Insert metadata
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO metadata (key, value) VALUES (?1, ?2)",
        ("version", index.version.to_string()),
    )?;
    tx.execute(
        "INSERT INTO metadata (key, value) VALUES (?1, ?2)",
        ("root", root.to_string_lossy().as_ref()),
    )?;
    tx.execute(
        "INSERT INTO metadata (key, value) VALUES (?1, ?2)",
        ("indexed_at", index.indexed_at.to_string()),
    )?;

    // Insert symbols
    {
        let mut stmt = tx.prepare(
            "INSERT INTO symbols (id, name, qualified_name, kind, file, start_line, end_line, visibility, signature, doc, parent)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )?;
        for s in index.symbols.values() {
            stmt.execute(rusqlite::params![
                s.id.0,
                s.name,
                s.qualified_name,
                s.kind.as_str(),
                s.file_path.to_string_lossy().as_ref(),
                s.span.start_line,
                s.span.end_line,
                format!("{:?}", s.visibility),
                s.signature.as_deref(),
                s.doc_comment.as_deref(),
                s.parent.as_ref().map(|p| p.0.as_str()),
            ])?;
        }
    }

    // Insert call edges
    if let Some(ref cg) = index.call_graph {
        let mut stmt = tx.prepare("INSERT INTO call_edges (caller, callee) VALUES (?1, ?2)")?;
        for &(from, to) in cg.all_edges() {
            stmt.execute(rusqlite::params![&cg.nodes[from], &cg.nodes[to]])?;
        }
    }

    // Insert files
    {
        let mut stmt =
            tx.prepare("INSERT INTO files (path, symbol_count, mtime) VALUES (?1, ?2, ?3)")?;
        for (file, ids) in &index.file_symbols {
            let mtime = index.file_mtimes.get(file).copied().unwrap_or(0);
            stmt.execute(rusqlite::params![
                file.to_string_lossy().as_ref(),
                ids.len(),
                mtime,
            ])?;
        }
    }

    // Create indexes for common queries
    tx.execute_batch(
        "CREATE INDEX idx_symbols_name ON symbols(name);
         CREATE INDEX idx_symbols_kind ON symbols(kind);
         CREATE INDEX idx_symbols_file ON symbols(file);
         CREATE INDEX idx_call_edges_caller ON call_edges(caller);
         CREATE INDEX idx_call_edges_callee ON call_edges(callee);",
    )?;

    tx.commit()?;

    let sym_count = index.symbols.len();
    let edge_count = index
        .call_graph
        .as_ref()
        .map(|cg| cg.all_edges().len())
        .unwrap_or(0);
    eprintln!(
        "Exported {} symbols + {} call edges to {}",
        sym_count,
        edge_count,
        db_path.display(),
    );

    Ok(())
}

#[cfg(feature = "native")]
fn dirs_cache_dir() -> anyhow::Result<std::path::PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| anyhow::anyhow!("Cannot determine home directory"))?;
    Ok(std::path::PathBuf::from(home)
        .join(".symlens")
        .join("indexes"))
}
