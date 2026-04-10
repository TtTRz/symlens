use clap::Parser;
use codelens::cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let root = cli.project_root.as_deref();

    match cli.command {
        Commands::Index(args) => codelens::commands::index::run(args, root),
        Commands::Search(args) => codelens::commands::search::run(args, root),
        Commands::Symbol(args) => codelens::commands::symbol::run(args, root),
        Commands::Outline(args) => codelens::commands::outline::run(args, root),
        Commands::Refs(args) => codelens::commands::refs::run(args, root),
        Commands::Callers(args) => codelens::commands::callers::run_callers(args, root),
        Commands::Callees(args) => codelens::commands::callers::run_callees(args, root),
        Commands::Lines(args) => codelens::commands::lines::run(args, root),
        Commands::Graph(args) => codelens::commands::graph::run(args, root),
        Commands::Watch(args) => codelens::commands::watch::run(args.path.as_deref().or(root)),
        Commands::Stats(args) => codelens::commands::stats::run(args, root),
        Commands::Blame(args) => codelens::commands::blame::run(args, root),
        Commands::Diff(args) => codelens::commands::diff::run(args, root),
        #[cfg(feature = "mcp")]
        Commands::Mcp => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(codelens::commands::mcp::server::run_mcp_server())
        }
    }
}
