mod cli;
mod commands;
mod context;
mod output;

use clap::Parser;
use cli::{Cli, Commands, DocsCommands, McpCommands, TypstCommands};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Doctor { json } => commands::doctor::run(json, cli.verbose),
        Commands::Gen(gen_cmd) => commands::scaffold::run(gen_cmd, cli.verbose),
        Commands::Generate { paper } => commands::generate::run(paper, cli.verbose),
        Commands::New { name, paper } => commands::new::run_new_project(name, paper, cli.verbose),
        Commands::Init { path, paper } => commands::new::run_init(path, paper, cli.verbose),
        Commands::Build { paper, full } => match paper {
            Some(id) => commands::build::run(id, full, cli.verbose),
            None => commands::build::run_all(full, cli.verbose),
        },
        Commands::Status { paper, json } => commands::status::run(paper, json, cli.verbose),
        Commands::Sync { docs, tools, all } => commands::sync::run(docs, tools, all, cli.verbose),
        Commands::Setup => commands::setup::run(cli.verbose),
        Commands::Lsp { command } => commands::lsp::run(command),
        Commands::Paper(args) => commands::paper::run(args.command, cli.verbose),
        Commands::Typst(typst_cmd) => match typst_cmd {
            TypstCommands::Link { force } => commands::typst::link::execute_link(force),
            TypstCommands::Install { version } => {
                commands::typst::install::execute_install(version, false)
            }
            TypstCommands::Version { json } => commands::typst::version::execute_version(json),
            TypstCommands::Versions { json } => commands::typst::versions::execute_versions(json),
            TypstCommands::Exec { args } => commands::typst::exec::execute_exec(args),
            TypstCommands::Docs(docs_cmd) => match docs_cmd {
                DocsCommands::Sync => commands::typst::docs::sync(cli.verbose),
                DocsCommands::Clear => commands::typst::docs::clear(cli.verbose),
                DocsCommands::Status { json } => commands::typst::docs::status(json, cli.verbose),
            },
        },
        Commands::Mcp(mcp_cmd) => match mcp_cmd {
            McpCommands::Stdio { root, offline } => commands::mcp::run_stdio(root, offline),
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
