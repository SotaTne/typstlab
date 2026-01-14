mod cli;
mod commands;
mod context;
mod output;

use clap::Parser;
use cli::{Cli, Commands, DocsCommands, TypstCommands};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Doctor { json } => commands::doctor::run(json, cli.verbose),
        Commands::Generate { paper } => commands::generate::run(paper, cli.verbose),
        Commands::Build { paper, full } => commands::build::run(paper, full, cli.verbose),
        Commands::Typst(typst_cmd) => match typst_cmd {
            TypstCommands::Link { force } => commands::typst::link::execute_link(force),
            TypstCommands::Install {
                version,
                from_cargo,
            } => commands::typst::install::execute_install(version, from_cargo),
            TypstCommands::Version { json } => commands::typst::version::execute_version(json),
            TypstCommands::Exec { args } => commands::typst::exec::execute_exec(args),
            TypstCommands::Docs(docs_cmd) => match docs_cmd {
                DocsCommands::Sync => commands::typst::docs::sync(cli.verbose),
                DocsCommands::Clear => commands::typst::docs::clear(cli.verbose),
                DocsCommands::Status { json } => commands::typst::docs::status(json, cli.verbose),
            },
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
