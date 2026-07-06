use clap::Parser;
use symlens::cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let root = cli.project_root.as_deref();
    let json = cli.json;
    let color = resolve_color(cli.no_color);

    if cli.verbose {
        // SAFETY: setting a new env var before any concurrent access
        unsafe {
            std::env::set_var("SYMLENS_VERBOSE", "1");
        }
    }

    let workspace = cli.workspace;

    // Route through daemon if --daemon flag is set
    if cli.daemon {
        match symlens::daemon::client::route_command(&cli) {
            Ok(Some(result)) => {
                println!("{}", serde_json::to_string_pretty(&result)?);
                return Ok(());
            }
            Ok(None) => {
                eprintln!("Warning: --daemon not supported for this command, running locally.");
                // Fall through to normal dispatch
            }
            Err(e) => {
                eprintln!("Daemon error: {}", e);
                std::process::exit(1);
            }
        }
    }

    match cli.command {
        Commands::Index(args) => symlens::commands::index::run(args, root, workspace),
        Commands::Search(args) => {
            symlens::commands::search::run(args, root, workspace, json, color)
        }
        Commands::Symbol(args) => {
            symlens::commands::symbol::run(args, root, workspace, json, color)
        }
        Commands::Outline(args) => {
            symlens::commands::outline::run(args, root, workspace, json, color)
        }
        Commands::Refs(args) => symlens::commands::refs::run(args, root, workspace, json, color),
        Commands::Callers(args) => {
            symlens::commands::callers::run_callers(args, root, workspace, json, color)
        }
        Commands::Callees(args) => {
            symlens::commands::callers::run_callees(args, root, workspace, json, color)
        }
        Commands::Lines(args) => symlens::commands::lines::run(args, root, workspace, color),
        Commands::Graph(args) => symlens::commands::graph::run(args, root, workspace, json),
        Commands::Watch(args) => {
            if args.serve {
                symlens::daemon::socket::serve_daemon(
                    args.path.as_deref().or(root),
                    workspace,
                    args.no_ignore,
                )
            } else {
                symlens::commands::watch::run(
                    args.path.as_deref().or(root),
                    workspace,
                    args.no_ignore,
                )
            }
        }
        Commands::Stats(args) => symlens::commands::stats::run(args, root, workspace, json),
        Commands::Blame(args) => symlens::commands::blame::run(args, root, workspace, json),
        Commands::Diff(args) => symlens::commands::diff::run(args, root, workspace, json, color),
        Commands::Export(args) => symlens::commands::export::run(args, root, workspace),
        Commands::Setup(args) => symlens::commands::setup::run(args, root),
        Commands::Completions(args) => symlens::commands::completions::run(args),
        Commands::Doctor => symlens::commands::doctor::run(root, workspace),
        Commands::Init => symlens::commands::init::run(root),
        #[cfg(feature = "mcp")]
        Commands::Mcp => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(symlens::commands::mcp::server::run_mcp_server())
        }
    }
}

/// Resolve color output based on: --no_color flag, NO_COLOR env, CLICOLOR_FORCE env, and isatty.
///
/// Precedence:
/// 1. --no_color → disable (explicit user request)
/// 2. NO_COLOR is set (non-empty) → disable (https://no-color.org/)
/// 3. CLICOLOR_FORCE is set (non-empty) → enable (override pipe detection)
/// 4. Auto-detect: enable if stdout is a terminal, disable if piped
fn resolve_color(no_color_flag: bool) -> bool {
    if no_color_flag {
        return false;
    }
    if std::env::var("NO_COLOR")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        return false;
    }
    if std::env::var("CLICOLOR_FORCE")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        return true;
    }
    atty_stdout()
}

/// Check if stdout is a terminal (for color auto-detection).
fn atty_stdout() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}
