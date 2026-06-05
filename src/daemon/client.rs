use crate::cli::{Cli, Commands, GraphCommand};
use crate::daemon::socket_path;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

/// Send a JSON-RPC request to the daemon and return the response.
pub fn send_request(
    socket_path: &Path,
    method: &str,
    params: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let stream = UnixStream::connect(socket_path).map_err(|e| {
        anyhow::anyhow!(
            "Cannot connect to daemon at {}: {}. Start it with `symlens watch --serve`.",
            socket_path.display(),
            e
        )
    })?;

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });

    // Write request
    let mut stream = stream;
    writeln!(stream, "{}", request)?;
    stream.flush()?;

    // Read response. Server uses blocking write_all, so the full JSON arrives as one line.
    let mut reader = BufReader::with_capacity(65536, stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;

    let response: serde_json::Value = serde_json::from_str(&response_line)?;

    if let Some(error) = response.get("error") {
        anyhow::bail!("Daemon error: {}", error["message"]);
    }

    Ok(response["result"].clone())
}

/// Compute the socket hash from root path without loading the full index.
/// This avoids ~7ms of index deserialization when the client only needs the socket path.
fn compute_socket_hash(root: &std::path::Path) -> String {
    let path_str = root.to_string_lossy();
    blake3::hash(path_str.as_bytes()).to_hex()[..16].to_string()
}

/// Resolve the socket path for the current project without loading the full index.
fn resolve_socket_path(
    root_override: Option<&str>,
    workspace_flag: bool,
) -> anyhow::Result<std::path::PathBuf> {
    let root = if let Some(explicit) = root_override {
        std::path::PathBuf::from(explicit).canonicalize()?
    } else {
        let cwd = std::env::current_dir()?;
        crate::index::storage::find_project_root(&cwd).unwrap_or(cwd)
    };

    let hash = compute_socket_hash(&root);
    Ok(socket_path(&hash, workspace_flag))
}

/// Map CLI command to RPC method and params. Returns None for commands
/// that should fall through to local execution.
pub fn route_command(cli: &Cli) -> anyhow::Result<Option<serde_json::Value>> {
    let (method, params) = match &cli.command {
        Commands::Search(args) => (
            "search",
            serde_json::json!({
                "query": args.query,
                "limit": args.limit,
                "kind": args.kind,
            }),
        ),
        Commands::Refs(args) => (
            "refs",
            serde_json::json!({
                "name": args.name,
                "limit": args.limit,
                "kind": args.kind,
            }),
        ),
        Commands::Callers(args) => (
            "callers",
            serde_json::json!({
                "name": args.name,
                "limit": args.limit,
            }),
        ),
        Commands::Callees(args) => (
            "callees",
            serde_json::json!({
                "name": args.name,
                "limit": args.limit,
            }),
        ),
        Commands::Outline(args) => (
            "outline",
            serde_json::json!({
                "file": args.file,
            }),
        ),
        Commands::Symbol(args) => (
            "symbol",
            serde_json::json!({
                "symbol_id": args.symbol_id,
                "source": args.source,
            }),
        ),
        Commands::Graph(args) => match &args.command {
            GraphCommand::Impact(ia) => (
                "impact",
                serde_json::json!({
                    "name": ia.name,
                    "depth": ia.depth,
                }),
            ),
            _ => return Ok(None),
        },
        Commands::Stats(_) => ("status", serde_json::json!({})),
        _ => return Ok(None),
    };

    let path = resolve_socket_path(cli.project_root.as_deref(), cli.workspace)?;
    let result = send_request(&path, method, params)?;
    Ok(Some(result))
}
