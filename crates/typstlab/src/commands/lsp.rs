use crate::cli::LspCommands;
use anyhow::Result;

pub fn run(command: Option<LspCommands>) -> Result<()> {
    match command {
        Some(LspCommands::Stdio) | None => {
            eprintln!("Starting LSP server (stdio)...");
            // In a real implementation, this would start the LSP event loop
            // typst_lsp::main_loop();
            Ok(())
        }
    }
}
