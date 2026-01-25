use super::types::DocsSearchArgs;
use crate::errors;
use crate::handlers::common::{ops, types::SearchConfig};
use crate::server::TypstlabServer;
use rmcp::{ErrorData as McpError, model::*};
use serde_json::json;
use tokio_util::sync::CancellationToken;
use typstlab_core::config::consts::search::{MAX_MATCHES, MAX_MATCHES_PER_FILE, MAX_SCAN_FILES};

pub(crate) async fn docs_search(
    server: &TypstlabServer,
    args: DocsSearchArgs,
    token: CancellationToken, // Added token
) -> Result<CallToolResult, McpError> {
    // Implementation note: DESIGN.md 5.10.5.1 requires a simple
    // substring match over each line, case-insensitive, without
    // interpreting whitespace as logical AND/OR.
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
                // Use docs_root relative path for search results (cross-platform)
                let rel_path = path
                    .strip_prefix(&docs_root_path)
                    .ok()?
                    .to_string_lossy()
                    .replace('\\', "/"); // Cross-platform consistency

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
