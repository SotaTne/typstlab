use anyhow::Result;
use std::env;
use typstlab_mcp::server::McpServer;

/// Run the MCP server over stdio
pub fn run_stdio() -> Result<()> {
    // Initialize server with current directory
    let root = env::current_dir()?;
    McpServer::run_stdio_server(root)
}
