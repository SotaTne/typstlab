use anyhow::{Context, Result};
use typstlab_mcp::TypstlabServer;

/// Run MCP server in stdio mode
pub fn run_stdio(root: Option<std::path::PathBuf>, offline: bool) -> Result<()> {
    let root = match root {
        Some(r) => typstlab_core::path::expand_tilde(&r),
        None => std::env::current_dir().context("Failed to get current directory")?,
    };

    // We need a tokio runtime for the server
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Failed to build tokio runtime")?;

    rt.block_on(async { TypstlabServer::run_stdio_server(root, offline).await })
}
