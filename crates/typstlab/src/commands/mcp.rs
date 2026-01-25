use anyhow::Result;
use std::env;
use typstlab_mcp::server::McpServer;

/// Run the MCP server over stdio
/// Run the MCP server over stdio
pub fn run_stdio(root: Option<std::path::PathBuf>, offline: bool) -> Result<()> {
    // Intialize server with provided root or current directory
    let root = root.unwrap_or(env::current_dir()?);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(McpServer::run_stdio_server(root, offline))?;
    Ok(())
}
