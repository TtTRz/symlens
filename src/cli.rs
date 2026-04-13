use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "codelens",
    version,
    about = "Token-efficient code intelligence powered by tree-sitter",
    long_about = "CodeLens indexes your codebase with tree-sitter and lets you fetch exactly \
                  the symbols you need — signatures, outlines, and call graphs — instead of \
                  reading entire files. Designed for AI agents (Claude Code) and humans alike."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Project root path (default: auto-detect via .git)
    #[arg(long = "root", global = true)]
    pub project_root: Option<String>,

    /// Output as JSON (applies to all commands)
    #[arg(long, global = true)]
    pub json: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Index the current project (or specified path)
    Index(IndexArgs),
    /// Search symbols by name, signature, or documentation
    Search(SearchArgs),
    /// Get detailed information about a specific symbol
    Symbol(SymbolArgs),
    /// Show file or project symbol outline
    Outline(OutlineArgs),
    /// Find references to a symbol (AST-level)
    Refs(RefsArgs),
    /// Show who calls a symbol
    Callers(CallersArgs),
    /// Show what a symbol calls
    Callees(CallersArgs),
    /// Get source code by line range
    Lines(LinesArgs),
    /// Dependency graph and impact analysis
    Graph(GraphArgs),
    /// Watch for file changes and auto-update index
    Watch(WatchCmdArgs),
    /// Show index statistics
    Stats(StatsArgs),
    /// Show git blame info for a symbol
    Blame(BlameArgs),
    /// Show changed symbols between git refs
    Diff(DiffArgs),
    /// Export index as JSON
    Export(ExportArgs),
    /// Install CodeLens integration into AI agents (Claude Code, OpenClaw, Cursor)
    Setup(SetupArgs),
    /// Start MCP (Model Context Protocol) server (requires --features mcp)
    #[cfg(feature = "mcp")]
    Mcp,
}

#[derive(clap::Args)]
pub struct IndexArgs {
    /// Path to the project root (default: current directory)
    pub path: Option<String>,

    /// Force re-index even if cache exists
    #[arg(long)]
    pub force: bool,

    /// Maximum number of files to index
    #[arg(long, default_value = "100000")]
    pub max_files: usize,

    /// Quiet mode (minimal output)
    #[arg(short, long)]
    pub quiet: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(clap::Args)]
pub struct SearchArgs {
    /// Search query string
    pub query: String,

    /// Filter by symbol kind (function, struct, method, enum, trait, ...)
    #[arg(long)]
    pub kind: Option<String>,

    /// Filter by file path prefix
    #[arg(long)]
    pub path: Option<String>,

    /// Maximum number of results
    #[arg(short, long, default_value = "20")]
    pub limit: usize,
}

#[derive(clap::Args)]
pub struct SymbolArgs {
    /// Symbol ID (e.g. "src/lib.rs::Engine::run#method")
    pub symbol_id: String,

    /// Include full source code
    #[arg(long)]
    pub source: bool,
}

#[derive(clap::Args)]
pub struct OutlineArgs {
    /// File path (omit for project outline)
    pub file: Option<String>,

    /// Show project-wide outline
    #[arg(long)]
    pub project: bool,

    /// Directory depth limit for project outline
    #[arg(long, default_value = "3")]
    pub depth: u32,

    /// Show summary counts only
    #[arg(long)]
    pub summary: bool,
}

#[derive(clap::Args)]
pub struct LinesArgs {
    /// File path (relative to project root)
    pub file: String,
    /// Start line number (1-based)
    pub start: u32,
    /// End line number (inclusive)
    pub end: u32,
}

#[derive(clap::Args)]
pub struct RefsArgs {
    /// Symbol name to find references for
    pub name: String,

    /// Filter by reference kind (call, type, import)
    #[arg(long)]
    pub kind: Option<String>,

    /// Scope search to a path prefix
    #[arg(long)]
    pub scope: Option<String>,

    /// Max results
    #[arg(short, long, default_value = "50")]
    pub limit: usize,
}

#[derive(clap::Args)]
pub struct CallersArgs {
    /// Symbol name
    pub name: String,

    /// Max results
    #[arg(short, long, default_value = "20")]
    pub limit: usize,
}

#[derive(clap::Args)]
pub struct GraphArgs {
    #[command(subcommand)]
    pub command: GraphCommand,
}

#[derive(Subcommand)]
pub enum GraphCommand {
    /// Analyze impact of modifying a symbol (blast radius)
    Impact(GraphImpactArgs),
    /// Show module dependency graph
    Deps(GraphDepsArgs),
    /// Find call path between two symbols
    Path(GraphPathArgs),
}

#[derive(clap::Args)]
pub struct GraphImpactArgs {
    /// Symbol name to analyze
    pub name: String,

    /// Max depth for transitive analysis
    #[arg(long, default_value = "3")]
    pub depth: usize,
}

#[derive(clap::Args)]
pub struct GraphDepsArgs {
    /// Subdirectory to focus on
    pub path: Option<String>,

    /// Output format (text, mermaid)
    #[arg(long, default_value = "text")]
    pub fmt: String,
}

#[derive(clap::Args)]
pub struct GraphPathArgs {
    /// Source symbol
    pub from: String,
    /// Target symbol
    pub to: String,
}

#[derive(clap::Args)]
pub struct WatchCmdArgs {
    /// Path to watch (default: current project)
    pub path: Option<String>,
}

#[derive(clap::Args)]
pub struct StatsArgs;

#[derive(clap::Args)]
pub struct BlameArgs {
    /// Symbol name or file::symbol ID
    pub name: String,
}

#[derive(clap::Args)]
pub struct DiffArgs {
    /// Git ref to compare from (default: HEAD~1)
    #[arg(long, default_value = "HEAD~1")]
    pub from: String,
    /// Git ref to compare to (default: HEAD)
    #[arg(long, default_value = "HEAD")]
    pub to: String,
    /// Only show symbols of this kind
    #[arg(long)]
    pub kind: Option<String>,
}

#[derive(clap::Args)]
pub struct ExportArgs {
    /// Export format: json (default), sqlite
    #[arg(long, default_value = "json")]
    pub format: String,

    /// Output file path (default: stdout for json, codelens.db for sqlite)
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct SetupArgs {
    /// Target agent: claude-code, openclaw, cursor
    pub agent: Option<String>,

    /// Install to all supported agents
    #[arg(long)]
    pub all: bool,

    /// Install globally (user-level) instead of project-level
    #[arg(short, long)]
    pub global: bool,

    /// Overwrite existing files
    #[arg(long)]
    pub force: bool,

    /// List supported agents
    #[arg(long)]
    pub list: bool,
}
