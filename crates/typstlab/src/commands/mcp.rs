use anyhow::{Context, Result};
use typstlab_mcp::TypstlabServer;

/// Run MCP server in stdio mode
pub fn run_stdio(root: Option<std::path::PathBuf>, offline: bool) -> Result<()> {
    let root = root.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // We need a tokio runtime for the server
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Failed to build tokio runtime")?;

    rt.block_on(async { TypstlabServer::run_stdio_server(root, offline).await })
}
