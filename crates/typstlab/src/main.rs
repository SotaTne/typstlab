mod cli;
mod commands;
mod context;
mod output;

use clap::Parser;
use cli::{Cli, Commands, DocsCommands, PaperCommands, TypstCommands};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Doctor { json } => commands::doctor::run(json, cli.verbose),
        Commands::Generate { paper } => commands::generate::run(paper, cli.verbose),
        Commands::New { name } => commands::new::run_new_project(name, cli.verbose),
        Commands::Paper(paper_cmd) => match paper_cmd {
            PaperCommands::New { id } => commands::new::run_new_paper(id, cli.verbose),
            PaperCommands::List { json } => commands::new::run_list_papers(json, cli.verbose),
        },
        Commands::Build { paper, full } => commands::build::run(paper, full, cli.verbose),
        Commands::Status { paper, json } => commands::status::run(paper, json, cli.verbose),
        Commands::Sync { apply } => commands::sync::run(apply, cli.verbose),
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
