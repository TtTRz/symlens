use clap::Parser;
use codelens::cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let root = cli.project_root.as_deref();
    let json = cli.json;
    let color = !cli.no_color && atty_stdout();

    match cli.command {
        Commands::Index(args) => codelens::commands::index::run(args, root),
        Commands::Search(args) => codelens::commands::search::run(args, root, json, color),
        Commands::Symbol(args) => codelens::commands::symbol::run(args, root, json, color),
        Commands::Outline(args) => codelens::commands::outline::run(args, root, json, color),
        Commands::Refs(args) => codelens::commands::refs::run(args, root, json, color),
        Commands::Callers(args) => codelens::commands::callers::run_callers(args, root, json),
        Commands::Callees(args) => codelens::commands::callers::run_callees(args, root, json),
        Commands::Lines(args) => codelens::commands::lines::run(args, root, color),
        Commands::Graph(args) => codelens::commands::graph::run(args, root, json),
        Commands::Watch(args) => codelens::commands::watch::run(args.path.as_deref().or(root)),
        Commands::Stats(args) => codelens::commands::stats::run(args, root, json),
        Commands::Blame(args) => codelens::commands::blame::run(args, root, json),
        Commands::Diff(args) => codelens::commands::diff::run(args, root, json, color),
        Commands::Export(args) => codelens::commands::export::run(args, root),
        Commands::Setup(args) => codelens::commands::setup::run(args, root),
        Commands::Completions(args) => codelens::commands::completions::run(args),
        Commands::Doctor => codelens::commands::doctor::run(root),
        Commands::Init => codelens::commands::init::run(root),
        #[cfg(feature = "mcp")]
        Commands::Mcp => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(codelens::commands::mcp::server::run_mcp_server())
        }
    }
}

/// Check if stdout is a terminal (for color auto-detection).
fn atty_stdout() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}
