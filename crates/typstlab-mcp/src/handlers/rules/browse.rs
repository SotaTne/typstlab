//! Rules browse機能

use super::types::{RulesBrowseArgs, resolve_rules_path};
use crate::errors;
use crate::server::TypstlabServer;
use rmcp::{ErrorData as McpError, model::*};
use serde_json::json;
use std::path::{Component, Path};
use tokio::fs;

/// rules_browseハンドラ
pub(crate) async fn rules_browse(
    server: &TypstlabServer,
    args: RulesBrowseArgs,
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

    let rules_root = server.context.project_root.join("rules");

    // DESIGN.md 5.10.5: browse系は {items: [], missing: bool, truncated?: bool}
    // rulesルート自体が存在しない場合
    if !rules_root.exists() {
        return Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&json!({
                "items": [],
                "truncated": false,
                "missing": true,
            }))
            .map_err(errors::from_display)?,
        )]));
    }

    let target = resolve_rules_path(&server.context.project_root, path).await?;

    // 存在しないパスもmissing=trueで返す（エラーにしない）
    if !target.exists() || !target.is_dir() {
        return Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&json!({
                "items": [],
                "truncated": false,
                "missing": true,
            }))
            .map_err(errors::from_display)?,
        )]));
    }
    let mut items = rules_browse_items(&target, &server.context.project_root).await?;
    items.sort_by_key(|i| {
        i.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    });
    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string(&json!({ "items": items })).map_err(errors::from_display)?,
    )]))
}

/// ディレクトリ内のルールファイル/ディレクトリをリストアップ
pub(crate) async fn rules_browse_items(
    target: &Path,
    project_root: &Path,
) -> Result<Vec<serde_json::Value>, McpError> {
    let mut items = Vec::new();
    let mut dir = fs::read_dir(target).await.map_err(errors::from_display)?;
    while let Some(entry) = dir.next_entry().await.map_err(errors::from_display)? {
        let file_type = entry.file_type().await.map_err(errors::from_display)?;
        if file_type.is_symlink() {
            continue;
        }

        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        let entry_type = if path.is_dir() {
            "directory"
        } else {
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            "file"
        };

        let relative_path = path
            .strip_prefix(project_root)
            .map_err(errors::from_display)?
            .to_string_lossy()
            .to_string();

        items.push(json!({
            "name": name,
            "type": entry_type,
            "path": relative_path,
        }));
    }
    Ok(items)
}
