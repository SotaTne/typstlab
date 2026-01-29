mod cli;
mod commands;

use crate::cli::{Cli, Commands, TreeSitterCommands};
use crate::commands::tree_sitter::{TreeSitterAction, TreeSitterCommand};
use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();

    match cli.command {
        Commands::TreeSitter(args) => {
            let action = match args.command {
                TreeSitterCommands::Setup => TreeSitterAction::Setup,
                TreeSitterCommands::Generate => TreeSitterAction::Generate,
                TreeSitterCommands::Verify(args) => TreeSitterAction::Verify {
                    ci: args.ci,
                    base_branch: args.base_branch,
                },
            };
            let cmd = TreeSitterCommand::new(action);
            use crate::commands::Command as _;
            cmd.run()?;
        }
    }

    Ok(())
}
