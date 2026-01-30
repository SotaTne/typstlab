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

    /// Generate _generated/ directories with rendered templates
    Generate {
        /// Paper ID to generate (if not specified, generates all papers)
        #[arg(short, long)]
        paper: Option<String>,
    },

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

    /// Paper management
    #[command(alias = "p")]
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

    /// Typst toolchain management
    #[command(subcommand)]
    Typst(TypstCommands),

    /// Show project status
    Status {
        /// Paper ID to filter status check
        #[arg(short, long)]
        paper: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Synchronize project to build-ready state
    Sync {
        /// Include documentation sync (network)
        #[arg(long)]
        docs: bool,

        /// Include toolchain resolution/install (network)
        #[arg(long)]
        tools: bool,

        /// Include everything (equivalent to --docs --tools)
        #[arg(long)]
        all: bool,
    },

    /// Run Language Server Protocol server
    Lsp {
        /// Run in stdio mode (default)
        #[command(subcommand)]
        command: Option<LspCommands>,
    },

    /// Run Model Context Protocol server
    #[command(subcommand)]
    Mcp(McpCommands),
}

#[derive(Args)]
pub struct PaperArgs {
    #[command(subcommand)]
    pub command: PaperCommands,
}

#[derive(Subcommand)]
pub enum PaperCommands {
    /// Create a new paper
    New {
        /// Paper ID (becomes directory name)
        id: String,
    },
    /// List all papers
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
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
    },

    /// Show Typst version information
    Version {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List all installed Typst versions
    Versions {
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

#[derive(Subcommand)]
pub enum LspCommands {
    /// Run in stdio mode
    Stdio,
}
