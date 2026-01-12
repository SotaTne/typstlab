//! CLI command structure using clap

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "typstlab")]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Check environment health
    Doctor {
        #[arg(long)]
        json: bool,
    },

    /// Typst toolchain management
    #[command(subcommand)]
    Typst(TypstCommands),
}

#[derive(Subcommand)]
pub enum TypstCommands {
    /// Documentation management
    #[command(subcommand)]
    Docs(DocsCommands),
}

#[derive(Subcommand)]
pub enum DocsCommands {
    /// Download Typst documentation
    Sync,

    /// Remove local documentation
    Clear,

    /// Show documentation status
    Status {
        #[arg(long)]
        json: bool,
    },
}
