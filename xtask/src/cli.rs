use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Typstlab automation tasks", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Tree-sitter related tasks
    #[command(name = "tree-sitter")]
    TreeSitter(TreeSitterArgs),
}

#[derive(Parser)]
pub struct TreeSitterArgs {
    #[command(subcommand)]
    pub command: TreeSitterCommands,
}

#[derive(Subcommand)]
pub enum TreeSitterCommands {
    /// Setup tree-sitter environment
    Setup,
    /// Generate tree-sitter parsers
    Generate,
    /// Verify tree-sitter parsers
    Verify(VerifyArgs),
}

#[derive(Parser)]
pub struct VerifyArgs {
    /// CI mode: strictly check commit coherence and output freshness
    #[arg(long)]
    pub ci: bool,

    /// Base branch for comparison (required in CI mode)
    #[arg(long, default_value = "origin/main")]
    pub base_branch: String,
}
