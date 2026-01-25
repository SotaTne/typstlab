use crate::errors;
use crate::handlers::common::{ops, types::SearchConfig};
use crate::handlers::{Safety, ToolExt};
use crate::server::TypstlabServer;
use futures_util::FutureExt;
use rmcp::{
    ErrorData as McpError,
    handler::server::common::FromContextPart,
    handler::server::router::tool::{ToolRoute, ToolRouter},
    handler::server::wrapper::Parameters,
    model::*,
    schemars, serde,
};
use serde_json::json;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

use typstlab_core::config::consts::search::{MAX_MATCHES, MAX_MATCHES_PER_FILE, MAX_SCAN_FILES};

pub struct DocsTool;

impl DocsTool {
    pub fn into_router(self) -> ToolRouter<TypstlabServer> {
        ToolRouter::new()
            .with_route(ToolRoute::new_dyn(Self::docs_browse_attr(), |mut ctx| {
                let server = ctx.service;
                let token = ctx.request_context.ct.clone(); // Use request token
                let args_res = Parameters::<DocsBrowseArgs>::from_context_part(&mut ctx);
                async move {
                    let Parameters(args) = args_res?;
                    Self::docs_browse(server, args, token).await
                }
                .boxed()
            }))
            .with_route(ToolRoute::new_dyn(Self::docs_search_attr(), |mut ctx| {
                let server = ctx.service;
                let token = ctx.request_context.ct.clone();
                let args_res = Parameters::<DocsSearchArgs>::from_context_part(&mut ctx);
                async move {
                    let Parameters(args) = args_res?;
                    Self::docs_search(server, args, token).await
                }
                .boxed()
            }))
    }

    fn docs_browse_attr() -> Tool {
        Tool::new(
            Cow::Borrowed("docs_browse"),
            "Browse documentation directory structure",
            rmcp::handler::server::common::schema_for_type::<DocsBrowseArgs>(),
        )
        .with_safety(Safety {
            network: false,
            reads: true,
            writes: false,
            writes_sot: false,
        })
    }

    fn docs_search_attr() -> Tool {
        Tool::new(
            Cow::Borrowed("docs_search"),
            "Search documentation files",
            rmcp::handler::server::common::schema_for_type::<DocsSearchArgs>(),
        )
        .with_safety(Safety {
            network: false,
            reads: true,
            writes: false,
            writes_sot: false,
        })
    }

    // テスト用: ハンドラ関数をpublicラッパー経由で公開
    pub async fn test_docs_browse(
        server: &TypstlabServer,
        args: DocsBrowseArgs,
    ) -> Result<CallToolResult, McpError> {
        Self::docs_browse(server, args, CancellationToken::new()).await
    }

    pub async fn test_docs_search(
        server: &TypstlabServer,
        args: DocsSearchArgs,
    ) -> Result<CallToolResult, McpError> {
        Self::docs_search(server, args, CancellationToken::new()).await
    }

    async fn docs_browse(
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

        let mut result = tokio::task::spawn_blocking(move || {
            ops::browse_dir_sync(
                &docs_root,
                &project_root,
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

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&result).map_err(errors::from_display)?,
        )]))
    }

    async fn docs_search(
        server: &TypstlabServer,
        args: DocsSearchArgs,
        token: CancellationToken, // Added token
    ) -> Result<CallToolResult, McpError> {
        // Validate query before processing
        let trimmed_query = args.query.trim();
        if trimmed_query.is_empty() {
            return Err(errors::invalid_input(
                "Search query cannot be empty or whitespace-only",
            ));
        }
        if trimmed_query.len() > 1000 {
            return Err(errors::invalid_input(
                "Search query too long (max 1000 characters)",
            ));
        }

        let docs_root = server.context.project_root.join(".typstlab/kb/typst/docs");
        if !docs_root.exists() {
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string(&json!({
                    "matches": [],
                    "truncated": false,
                    "missing": true,
                }))
                .map_err(errors::from_display)?,
            )]));
        }

        let query_lowercase = trimmed_query.to_lowercase();
        let docs_root_path = docs_root.clone();
        let project_root_path = server.context.project_root.clone();

        let config = SearchConfig::new(MAX_SCAN_FILES, MAX_MATCHES, vec!["md".to_string()]);

        let _guard = token.clone().drop_guard();

        // Run search in blocking thread
        let result = tokio::task::spawn_blocking(move || {
            ops::search_dir_sync(
                &docs_root_path,
                &project_root_path,
                &config,
                token,
                |path, content| {
                    let mut file_matches = Vec::new();
                    // Use project root relative path for consistency with rules_search
                    let rel_path = path
                        .strip_prefix(&project_root_path)
                        .ok()?
                        .to_string_lossy();

                    for (line_index, line) in content.lines().enumerate() {
                        if line.to_lowercase().contains(&query_lowercase) {
                            file_matches.push(json!({
                                "path": rel_path,
                                "line": line_index + 1,
                                "content": line.trim(),
                            }));

                            if file_matches.len() >= MAX_MATCHES_PER_FILE {
                                break;
                            }
                        }
                    }
                    if file_matches.is_empty() {
                        None
                    } else {
                        Some(file_matches)
                    }
                },
            )
        })
        .await
        .map_err(|e| errors::internal_error(format!("Search task panicked: {}", e)))??;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&result).map_err(errors::from_display)?,
        )]))
    }
}

pub(crate) async fn resolve_docs_path(
    project_root: &Path,
    docs_root: &Path,
    requested: &Path,
) -> Result<PathBuf, McpError> {
    use crate::handlers::common::{ops::check_entry_safety, path::resolve_safe_path};
    // First, perform the standard path validation relative to docs_root
    let resolved = resolve_safe_path(docs_root, requested).await?;

    // Additional defense: ensure the resolved path stays under the project root even if docs_root
    // itself is a symlink pointing outside. Only check if path exists.
    if resolved.exists() {
        check_entry_safety(&resolved, project_root)?;
    }

    Ok(resolved)
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct DocsBrowseArgs {
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct DocsSearchArgs {
    pub query: String,
}

#[cfg(test)]
mod tests;
