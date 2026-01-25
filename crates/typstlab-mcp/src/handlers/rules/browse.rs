//! Rules browse機能

use super::types::{RulesBrowseArgs, resolve_rules_path};
use crate::errors;
use crate::server::TypstlabServer;
use rmcp::{ErrorData as McpError, model::*};
use serde_json::json;
use std::path::{Component, Path};

/// rules_browseハンドラ
pub(crate) async fn rules_browse(
    server: &TypstlabServer,
    args: RulesBrowseArgs,
    token: tokio_util::sync::CancellationToken,
) -> Result<CallToolResult, McpError> {
    // Validate path before using it
    let path = Path::new(&args.path);
    if typstlab_core::path::has_absolute_or_rooted_component(path) {
        return Err(errors::path_escape(format!(
            "Path '{}' cannot be absolute or rooted",
            args.path
        )));
    }
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(errors::path_escape(format!(
            "Path '{}' cannot contain ..",
            args.path
        )));
    }

    let is_rules_path = path.starts_with("rules");
    let is_papers_path = path.starts_with("papers");
    if !is_rules_path && !is_papers_path {
        let candidate = server.context.project_root.join(path);
        if !candidate.exists() {
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string(&json!({
                    "items": [],
                    "missing": true,
                    "truncated": false,
                }))
                .map_err(errors::from_display)?,
            )]));
        }
    }

    // パス解決
    // resolve_rules_path のエラーは validation error なので伝播させる
    let target = resolve_rules_path(&server.context.project_root, path).await?;
    let project_root = server.context.project_root.clone();

    // ディレクトリ存在確認
    if !target.exists() {
        return Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&json!({
                "items": [],
                "missing": true,
                "truncated": false,
            }))
            .map_err(errors::from_display)?,
        )]));
    }

    if target.is_file() {
        return Err(errors::invalid_input("Path must point to a directory"));
    }

    // ops::browse_dir_syncを利用して安全にブラウズ
    // resolve_rules_pathでチェック済みだが、念のため再チェックになる（コストは低い）
    // targetは絶対パス（resolve_rules_pathが返す）

    // browse_dir_syncは (root, project_root, relative_path, ...)
    // ここでは target を root として relative_path=None で呼ぶか、
    // rules_root を root として relative_path を渡すか。
    // resolve_rules_pathは rules/... または papers/... を解決するので、
    // target を root として渡すのが適切。

    let browse_res = tokio::task::spawn_blocking(move || {
        crate::handlers::common::ops::browse_dir_sync(
            &target,
            &project_root,
            None,
            &["md".to_string()],
            1000,
            token,
        )
    })
    .await
    .map_err(|e| errors::internal_error(format!("Browse task panicked: {}", e)))??;

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string(&json!({
             "items": browse_res.items,
             "missing": browse_res.missing,
             "truncated": browse_res.truncated,
        }))
        .map_err(errors::from_display)?,
    )]))
}

// rules_browse_items deleted
