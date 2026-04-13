use crate::cli::CompletionsArgs;
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::io;

pub fn run(args: CompletionsArgs) -> anyhow::Result<()> {
    let shell = match args.shell.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        other => anyhow::bail!("Unsupported shell: '{}'. Supported: bash, zsh, fish", other),
    };

    let mut cmd = crate::cli::Cli::command();
    generate(shell, &mut cmd, "symlens", &mut io::stdout());
    Ok(())
}
