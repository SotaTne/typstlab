//! Rules list機能

use super::types::RulesListArgs;
use crate::errors;
use crate::server::TypstlabServer;
use rmcp::{ErrorData as McpError, model::*};
use serde_json::json;
use tokio::fs;
use typstlab_core::config::consts::search::{MAX_FILE_BYTES, MAX_SCAN_FILES};
use walkdir::WalkDir;

/// rules_listハンドラ
pub(crate) async fn rules_list(
    server: &TypstlabServer,
    args: RulesListArgs,
) -> Result<CallToolResult, McpError> {
    // Validate paper_id before using it in path construction
    if let Some(paper_id) = args.paper_id.as_ref() {
        typstlab_core::path::validate_paper_id(paper_id).map_err(errors::from_core_error)?;
    }

    let mut dirs = Vec::new();
    if args.include_root {
        dirs.push((server.context.project_root.join("rules"), "root"));
    }

    if let Some(paper_id) = args.paper_id.as_ref() {
        dirs.push((
            server
                .context
                .project_root
                .join("papers")
                .join(paper_id)
                .join("rules"),
            "paper",
        ));
    }

    let mut files = Vec::new();
    let mut scanned = 0usize;
    let mut truncated = false;
    for (dir, origin) in dirs {
        if !dir.exists() {
            continue;
        }

        for entry in WalkDir::new(&dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_type = entry.file_type();
            if file_type.is_symlink() || !file_type.is_file() {
                continue;
            }

            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }

            scanned += 1;
            if scanned > MAX_SCAN_FILES {
                truncated = true;
                files.clear();
                break;
            }

            let metadata = fs::metadata(path).await.map_err(errors::from_display)?;
            if metadata.len() > MAX_FILE_BYTES {
                continue;
            }

            let rel_path = path
                .strip_prefix(&server.context.project_root)
                .map_err(errors::from_display)?
                .to_string_lossy();
            files.push(json!({
                "path": rel_path,
                "origin": origin,
            }));
        }
        if truncated {
            break;
        }
    }

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string(&json!({
            "files": files,
            "truncated": truncated,
        }))
        .map_err(errors::from_display)?,
    )]))
}
