use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;

#[derive(Parser)]
#[command(name = "xtask")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[command(rename_all = "snake_case")]
enum Commands {
    /// Check version resolver JSON files for schema validity
    JsonCheck,
    /// Check Typst docs.json files against docs_parser schema.rs
    CheckDocsSchema { files: Vec<PathBuf> },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::JsonCheck => commands::json_check::run(),
        Commands::CheckDocsSchema { files } => commands::check_docs_schema::run(&files),
    }
}
