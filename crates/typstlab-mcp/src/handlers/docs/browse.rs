use super::types::{DocsBrowseArgs, resolve_docs_path};
use crate::errors;
use crate::handlers::common::ops;
use crate::server::TypstlabServer;
use rmcp::{ErrorData as McpError, model::*};
use serde_json::json;
use tokio_util::sync::CancellationToken;

pub(crate) async fn docs_browse(
    server: &TypstlabServer,
    args: DocsBrowseArgs,
    token: CancellationToken, // Added token
) -> Result<CallToolResult, McpError> {
    let docs_root = server.context.project_root.join(".typstlab/kb/typst/docs");

    // docsルート自体が存在しない場合
    if !docs_root.exists() {
        return Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&json!({
                "items": [],
                "truncated": false,
                "missing": true,
            }))
            .map_err(errors::from_display)?,
        )]));
    }

    let _guard = token.clone().drop_guard();

    let path_arg = args.path.clone();
    let project_root = server.context.project_root.clone();

    // Resolve and validate path
    if let Some(ref path_str) = path_arg {
        let requested_path = std::path::Path::new(path_str);
        let resolved =
            resolve_docs_path(&server.context.project_root, &docs_root, requested_path).await?;
        if resolved.is_file() {
            return Err(errors::invalid_input("Path must point to a directory"));
        }
    }

    let docs_root_for_browse = docs_root.clone();
    let project_root_for_browse = project_root.clone();
    let mut result = tokio::task::spawn_blocking(move || {
        ops::browse_dir_sync(
            &docs_root_for_browse,
            &project_root_for_browse,
            path_arg.as_deref(),
            &["md".to_string()],
            1000,
            token,
        )
    })
    .await
    .map_err(|e| errors::internal_error(format!("Browse task panicked: {}", e)))??;

    // docs固有ロジック: 隠しファイル除外
    result.items.retain(|item| !item.name.starts_with('.'));
    let prefix = docs_root
        .strip_prefix(&project_root)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"));
    if let Some(prefix) = prefix {
        let prefix = format!("{}/", prefix);
        for item in &mut result.items {
            if let Some(stripped) = item.path.strip_prefix(&prefix) {
                item.path = format!("docs/{}", stripped);
            }
        }
    }

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string(&result).map_err(errors::from_display)?,
    )]))
}
