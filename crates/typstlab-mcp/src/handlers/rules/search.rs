//! Rules search機能

use super::types::{RulesSearchArgs, collect_search_dirs};
use crate::errors;
use crate::handlers::LineRange;
use crate::handlers::common::{ops, types::SearchConfig};
use crate::handlers::rules::RulesMatches;
use crate::server::TypstlabServer;
use rmcp::{ErrorData as McpError, model::*};
use serde_json::json;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;
use typstlab_core::config::consts::search::{MAX_MATCHES, MAX_MATCHES_PER_FILE, MAX_SCAN_FILES};

struct ExtendedSearchResult {
    matches: Vec<RulesMatches>,
    truncated: bool,
    #[allow(dead_code)]
    scanned_files: usize,
    missing: bool,
}

/// rules_searchハンドラ
pub(crate) async fn rules_search(
    server: &TypstlabServer,
    args: RulesSearchArgs,
    token: CancellationToken,
) -> Result<CallToolResult, McpError> {
    // Implementation note: follows DESIGN.md 5.10.5.1 – per-line substring
    // matching, case-insensitive, no boolean parsing.
    // Validate paper_id before using it in path construction
    if let Some(paper_id) = args.paper_id.as_ref() {
        typstlab_core::path::validate_paper_id(paper_id).map_err(errors::from_core_error)?;
    }

    // Validate and sanitize query
    let query = args.query.trim();
    if query.is_empty() {
        return Err(errors::invalid_input(
            "Search query cannot be empty or whitespace-only",
        ));
    }
    if query.len() > 1000 {
        return Err(errors::invalid_input(
            "Search query too long (max 1000 characters)",
        ));
    }

    let search_dirs = collect_search_dirs(&server.context.project_root, &args);
    let query_lowercase = query.to_lowercase();
    let project_root = server.context.project_root.clone();

    // Cancellation setup
    let _guard = token.clone().drop_guard();

    let outcome =
        search_rules_dirs_blocking(project_root, query_lowercase, search_dirs, token, args).await?;

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string(&json!({
            "matches": outcome.matches,
            "truncated": outcome.truncated,
            "missing": outcome.missing,
        }))
        .map_err(errors::from_display)?,
    )]))
}

// blocking wrapper
// blocking wrapper
async fn search_rules_dirs_blocking(
    project_root: PathBuf,
    query: String,
    search_dirs: Vec<(std::path::PathBuf, &'static str)>,
    token: CancellationToken,
    pure_query: RulesSearchArgs,
) -> Result<ExtendedSearchResult, McpError> {
    let res = tokio::task::spawn_blocking(move || {
        let mut total_matches = Vec::new();
        let mut total_scanned = 0usize;
        let mut truncated = false;
        let mut missing = false;
        let mut all_missing = true;

        // Check if any required directory is missing.
        // For search, we iterate over target dirs. If a dir doesn't exist, search_dir_sync might just return empty?
        // Let's verify existence first.

        for (dir, _) in &search_dirs {
            if dir.exists() {
                all_missing = false;
            }
        }
        if all_missing {
            missing = true;
        }

        for (dir, origin) in search_dirs {
            // Check remaining quota
            if total_scanned >= MAX_SCAN_FILES {
                // DESIGN.md 5.10.9: Clear all results when file scan limit is reached
                total_matches.clear();
                truncated = true;
                break;
            }
            let remaining_quota = MAX_SCAN_FILES - total_scanned;

            let config = SearchConfig::new(
                remaining_quota,
                MAX_MATCHES, // logic: max matches per directory or total?
                // Specification usually implies total matches limit.
                // If we have existing matches, we should reduce config.max_matches?
                // Ops applies max_matches to the list.
                // If we pass (MAX_MATCHES - current_matches), ops will return up to that.
                // Yes, we should throttle matches too.
                vec!["md".to_string()],
            );

            if total_matches.len() >= MAX_MATCHES {
                truncated = true;
                break;
            }
            let matches_quota = MAX_MATCHES - total_matches.len();
            let mut sub_config = config.clone();
            sub_config.max_matches = matches_quota;

            let outcome = ops::search_dir_sync(
                &dir,
                &project_root,
                &sub_config,
                token.clone(),
                |path, content, metadata| {
                    // Mapper logic
                    let mut file_matches: Vec<RulesMatches> = Vec::new();
                    let lines: Vec<&str> = content.lines().collect();

                    // Relative path from PROJECT ROOT
                    let rel_path = path
                        .strip_prefix(&project_root)
                        .ok()?
                        .to_string_lossy()
                        .replace('\\', "/");

                    let mtime = metadata
                        .modified()
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();

                    for (line_index, line) in lines.iter().enumerate() {
                        if line.to_lowercase().contains(&query) {
                            let start = line_index.saturating_sub(2);
                            let end = (line_index + 2).min(lines.len().saturating_sub(1));
                            let excerpt = lines[start..=end].join("\n");

                            file_matches.push(RulesMatches {
                                uri: format!("typstlab://rules/{}", rel_path),
                                path: rel_path.clone(),
                                line: line_index + 1,
                                preview: excerpt,
                                line_range: LineRange { start, end },
                                origin,
                                mtime,
                            });

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
                pure_query.clone(),
            );

            let res = match outcome {
                Ok(r) => r,
                Err(e) => return Err(e),
            };

            let matches_empty = res.matches.is_empty();
            let scanned_files = res.scanned_files;
            let truncated_local = res.truncated;

            // Aggregate results
            total_matches.extend(res.matches);
            total_scanned += scanned_files;

            if truncated_local {
                // DESIGN.md 5.10.9: clear results only when file scan limit is reached.
                // search_dir_sync clears matches on MAX_SCAN_FILES, but keeps matches on MAX_MATCHES.
                if matches_empty && scanned_files >= sub_config.max_files {
                    total_matches.clear();
                }
                truncated = true;
                // If one dir is truncated, global result is truncated.
                // Should we continue to next dir? Usually no, because we hit limit.
                break;
            }
        }

        Ok(ExtendedSearchResult {
            matches: total_matches,
            truncated,
            scanned_files: total_scanned,
            missing,
        })
    })
    .await;

    match res {
        Ok(search_res) => search_res,
        Err(join_err) => {
            if join_err.is_cancelled() {
                Err(errors::request_cancelled())
            } else {
                Err(errors::internal_error(format!(
                    "Search task panicked: {:?}",
                    join_err
                )))
            }
        }
    }
}
