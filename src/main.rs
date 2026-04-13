use clap::Parser;
use symlens::cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let root = cli.project_root.as_deref();
    let json = cli.json;
    let color = !cli.no_color && atty_stdout();

    match cli.command {
        Commands::Index(args) => symlens::commands::index::run(args, root),
        Commands::Search(args) => symlens::commands::search::run(args, root, json, color),
        Commands::Symbol(args) => symlens::commands::symbol::run(args, root, json, color),
        Commands::Outline(args) => symlens::commands::outline::run(args, root, json, color),
        Commands::Refs(args) => symlens::commands::refs::run(args, root, json, color),
        Commands::Callers(args) => symlens::commands::callers::run_callers(args, root, json),
        Commands::Callees(args) => symlens::commands::callers::run_callees(args, root, json),
        Commands::Lines(args) => symlens::commands::lines::run(args, root, color),
        Commands::Graph(args) => symlens::commands::graph::run(args, root, json),
        Commands::Watch(args) => symlens::commands::watch::run(args.path.as_deref().or(root)),
        Commands::Stats(args) => symlens::commands::stats::run(args, root, json),
        Commands::Blame(args) => symlens::commands::blame::run(args, root, json),
        Commands::Diff(args) => symlens::commands::diff::run(args, root, json, color),
        Commands::Export(args) => symlens::commands::export::run(args, root),
        Commands::Setup(args) => symlens::commands::setup::run(args, root),
        Commands::Completions(args) => symlens::commands::completions::run(args),
        Commands::Doctor => symlens::commands::doctor::run(root),
        Commands::Init => symlens::commands::init::run(root),
        #[cfg(feature = "mcp")]
        Commands::Mcp => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(symlens::commands::mcp::server::run_mcp_server())
        }
    }
}

/// Check if stdout is a terminal (for color auto-detection).
fn atty_stdout() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}
