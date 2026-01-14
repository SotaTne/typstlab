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
    /// Link to system or managed Typst
    Link {
        /// Force re-resolution even if cached
        #[arg(short, long)]
        force: bool,
    },

    /// Install Typst version
    Install {
        /// Version to install (e.g., "0.12.0")
        version: String,

        /// Install from cargo instead of GitHub
        #[arg(long)]
        from_cargo: bool,
    },

    /// Show Typst version information
    Version {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Execute Typst binary with arguments
    #[command(trailing_var_arg = true)]
    Exec {
        /// Arguments to pass to Typst (after --)
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },

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
