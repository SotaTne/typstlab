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
    /// Test tree-sitter parsers
    Test,
}

#[derive(Parser)]
pub struct VerifyArgs {}
