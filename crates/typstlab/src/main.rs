mod cli;
mod commands;
mod context;
mod output;

use clap::Parser;
use cli::{Cli, Commands, McpCommands};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Doctor { json } => commands::doctor::run(json, cli.verbose),
        Commands::Gen(gen_cmd) => commands::scaffold::run(gen_cmd, cli.verbose),

        Commands::New { name, paper } => commands::new::run_new_project(name, paper, cli.verbose),
        Commands::Init { path, paper } => commands::new::run_init(path, paper, cli.verbose),
        Commands::Build { paper, full } => match paper {
            Some(id) => commands::build::run(id, full, cli.verbose),
            None => commands::build::run_all(full, cli.verbose),
        },
        Commands::Setup => commands::setup::run(cli.verbose),
        Commands::Paper(args) => commands::paper::run(args.command, cli.verbose),

        Commands::Mcp(mcp_cmd) => match mcp_cmd {
            McpCommands::Stdio { root, offline } => commands::mcp::run_stdio(root, offline),
        },
        Commands::Typst(args) => commands::typst::run(args.command, cli.verbose),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
