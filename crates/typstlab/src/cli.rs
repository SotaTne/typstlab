//! CLI command structure using clap

use clap::{Args, Parser, Subcommand};

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

    /// Setup environment (install + sync --all)
    Setup,

    /// Create new project
    New {
        /// Project name (becomes directory name)
        name: String,

        /// Paper ID to generate immediately
        #[arg(long)]
        paper: Option<String>,
    },

    /// Initialize project in existing directory
    Init {
        /// Path to initialize (defaults to current directory)
        path: Option<String>,

        /// Paper ID to generate immediately
        #[arg(long)]
        paper: Option<String>,
    },

    /// Scaffold new project items (paper, template, lib)
    #[command(subcommand)]
    Gen(GenCommands),

    /// Paper management (list, etc.)
    Paper(PaperArgs),

    /// Build paper to PDF
    Build {
        /// Paper ID to build (if not specified, builds all papers)
        #[arg(short, long)]
        paper: Option<String>,

        /// Force regenerate _generated/ before build
        #[arg(long)]
        full: bool,
    },

    /// Run Model Context Protocol server
    #[command(subcommand)]
    Mcp(McpCommands),

    /// Management of the Typst toolchain
    Typst(TypstArgs),
}

#[derive(Args, Debug)]
pub struct TypstArgs {
    #[command(subcommand)]
    pub command: TypstSubcommands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum TypstSubcommands {
    /// Install Tipst of the specified version or the version in typstlab.toml
    Install {
        /// Version to install (e.g. 0.12.0)
        version: Option<String>,
    },
    /// Show current Typst version
    Version {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List all managed and system Typst versions
    Versions {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Execute Typst binary directly
    Exec {
        /// Arguments to pass to Typst
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Typst documentation management
    #[command(subcommand)]
    Docs(DocsCommands),
}

#[derive(Subcommand, Debug, Clone)]
pub enum DocsCommands {
    /// Synchronize Typst documentation
    Sync,
    /// Clear local Typst documentation
    Clear,
    /// Show documentation sync status
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum GenCommands {
    /// Create a new paper
    Paper {
        /// Paper ID (becomes directory name)
        id: String,
        /// Template to use (local path or typst package e.g., @preview/jaconf)
        #[arg(short, long)]
        template: Option<String>,
        /// Title of the paper (optional)
        #[arg(long)]
        title: Option<String>,
    },
    /// Create a new template
    Template {
        /// Template name (becomes directory name in templates/)
        name: String,
    },
    /// Create a new library (stub)
    Lib {
        /// Library name
        name: String,
    },
}

#[derive(Args)]
pub struct PaperArgs {
    #[command(subcommand)]
    pub command: PaperCommands,
}

#[derive(Subcommand)]
pub enum PaperCommands {
    /// List all papers
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum McpCommands {
    /// Run the MCP server over stdio
    Stdio {
        /// Project root directory (optional, defaults to current directory)
        #[arg(long)]
        root: Option<std::path::PathBuf>,
        /// Disable tools that require network access
        #[arg(long)]
        offline: bool,
    },
}
