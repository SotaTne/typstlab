use super::types::{DocsGetArgs, resolve_docs_path};
use crate::errors;
use crate::handlers::common::ops;
use crate::server::TypstlabServer;
use rmcp::{ErrorData as McpError, model::*};
use std::path::Path;

pub(crate) async fn docs_get(
    server: &TypstlabServer,
    args: DocsGetArgs,
) -> Result<CallToolResult, McpError> {
    let docs_root = server.context.project_root.join(".typstlab/kb/typst/docs");
    let requested_path = Path::new(&args.path);

    let target =
        resolve_docs_path(&server.context.project_root, &docs_root, requested_path).await?;
    let project_root = server.context.project_root.clone();

    // Removed "move" keyword as docs_root/requested_path are dropped; only target/project_root needed
    let content =
        tokio::task::spawn_blocking(move || ops::read_markdown_file_sync(&target, &project_root))
            .await
            .map_err(|e| errors::internal_error(format!("Read task panicked: {}", e)))??;

    Ok(CallToolResult::success(vec![Content::text(content)]))
}
