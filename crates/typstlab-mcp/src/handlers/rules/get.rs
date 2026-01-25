//! Rules get/page機能

use super::types::{RulesGetArgs, RulesPageArgs, enforce_rules_file_size, resolve_rules_path};
use crate::errors;
use crate::server::TypstlabServer;
use rmcp::{ErrorData as McpError, model::*};
use serde_json::json;
use std::path::Path;
use tokio::fs;

/// rules_getハンドラ
pub(crate) async fn rules_get(
    server: &TypstlabServer,
    args: RulesGetArgs,
) -> Result<CallToolResult, McpError> {
    let path = Path::new(&args.path);
    let target = resolve_rules_path(&server.context.project_root, path).await?;

    if !target.exists() || !target.is_file() {
        return Err(errors::invalid_params(format!(
            "File not found or not a valid rule file: {}",
            args.path
        )));
    }

    if target.extension().and_then(|ext| ext.to_str()) != Some("md") {
        return Err(errors::invalid_params("File must be a markdown (.md) file"));
    }

    enforce_rules_file_size(&target).await?;

    let content = fs::read_to_string(target)
        .await
        .map_err(errors::from_display)?;
    Ok(CallToolResult::success(vec![Content::text(content)]))
}

/// rules_pageハンドラ
pub(crate) async fn rules_page(
    server: &TypstlabServer,
    args: RulesPageArgs,
) -> Result<CallToolResult, McpError> {
    let path = Path::new(&args.path);
    let target = resolve_rules_path(&server.context.project_root, path).await?;

    if !target.exists() || !target.is_file() {
        return Err(errors::invalid_params(format!(
            "File not found or not a valid rule file: {}",
            args.path
        )));
    }

    if target.extension().and_then(|ext| ext.to_str()) != Some("md") {
        return Err(errors::invalid_params("File must be a markdown (.md) file"));
    }

    enforce_rules_file_size(&target).await?;

    let content = fs::read_to_string(target)
        .await
        .map_err(errors::from_display)?;
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();

    let offset = args.offset.unwrap_or(0);
    let limit = args.limit.unwrap_or(100);

    let end = (offset + limit).min(total);
    if offset >= total {
        return Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&json!({
                "content": "",
                "offset": offset,
                "limit": limit,
                "total": total,
            }))
            .map_err(errors::from_display)?,
        )]));
    }

    let slice = &lines[offset..end];
    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string(&json!({
            "content": slice.join("\n"),
            "offset": offset,
            "limit": limit,
            "total": total,
        }))
        .map_err(errors::from_display)?,
    )]))
}
