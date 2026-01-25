//! Rules list機能

use super::types::RulesListArgs;
use crate::errors;
use crate::handlers::common::ops;
use crate::server::TypstlabServer;
use rmcp::{ErrorData as McpError, model::*};
use serde_json::json;
use tokio_util::sync::CancellationToken;
use typstlab_core::config::consts::search::{MAX_FILE_BYTES, MAX_SCAN_FILES};
use walkdir::WalkDir;

/// rules_listハンドラ
pub(crate) async fn rules_list(
    server: &TypstlabServer,
    args: RulesListArgs,
    token: CancellationToken,
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

    let project_root = server.context.project_root.clone();
    let _guard = token.clone().drop_guard();

    // Blocking task
    let result = tokio::task::spawn_blocking(move || {
        let mut files = Vec::new();
        let mut scanned = 0usize;
        let mut truncated = false;

        for (dir, origin) in dirs {
            if !dir.exists() {
                continue;
            }

            // Check dir safety
            if ops::check_entry_safety(&dir, &project_root).is_err() {
                continue;
            }

            if token.is_cancelled() {
                return Err(errors::request_cancelled());
            }

            for entry in WalkDir::new(&dir).follow_links(false).into_iter() {
                if token.is_cancelled() {
                    return Err(errors::request_cancelled());
                }

                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::debug!("WalkDir error: {}", e);
                        continue;
                    }
                };

                // Limits check
                if entry.file_type().is_dir() {
                    continue;
                }

                // Safety check
                let path = entry.path();
                if let Err(e) = ops::check_entry_safety(path, &project_root) {
                    tracing::debug!("Unsafe entry {:?}: {:?}", path, e);
                    continue;
                }

                if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                    continue;
                }

                scanned += 1;
                if scanned > MAX_SCAN_FILES {
                    truncated = true;
                    // For listing, we clear logic?
                    // Search logic clears. List logic also clears in previous impl.
                    files.clear();
                    break;
                }

                let metadata = match std::fs::metadata(path) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                if metadata.len() > MAX_FILE_BYTES {
                    continue;
                }

                let rel_path = path
                    .strip_prefix(&project_root)
                    .map_err(|e| errors::internal_error(e.to_string()))?
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

        Ok(json!({
            "files": files,
            "truncated": truncated
        }))
    })
    .await
    .map_err(|e| errors::internal_error(format!("List task panicked: {}", e)))??;

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string(&result).map_err(errors::from_display)?,
    )]))
}
