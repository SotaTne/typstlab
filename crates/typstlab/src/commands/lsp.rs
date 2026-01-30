//! LSP command - start Language Server
//!
//! In v0.1, this is a placeholder/stub.

use anyhow::Result;

use crate::cli::LspCommands;

/// Run LSP server
pub fn run(command: Option<LspCommands>) -> Result<()> {
    match command {
        Some(LspCommands::Stdio) | None => {
            println!("LSP server running on stdio (stub)");
        }
    }
    Ok(())
}
