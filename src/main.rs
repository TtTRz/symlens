mod cli;
mod commands;
mod graph;
mod index;
mod model;
mod output;
mod parser;
mod refs;
mod search;
mod util;

use clap::Parser;
use cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Index(args) => commands::index::run(args),
        Commands::Search(args) => commands::search::run(args),
        Commands::Symbol(args) => commands::symbol::run(args),
        Commands::Outline(args) => commands::outline::run(args),
        Commands::Refs(args) => commands::refs::run(args),
        Commands::Callers(args) => commands::callers::run_callers(args),
        Commands::Callees(args) => commands::callers::run_callees(args),
        Commands::Lines(args) => commands::lines::run(args),
        Commands::Graph(args) => commands::graph::run(args),
        Commands::Watch(args) => commands::watch::run(args.path),
        Commands::Stats(args) => commands::stats::run(args),
    }
}
