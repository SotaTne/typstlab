//! Rules search機能

use super::types::{RulesSearchArgs, collect_search_dirs};
use crate::errors;
use crate::handlers::LineRange;
use crate::handlers::common::{ops, types::SearchConfig};
use crate::handlers::rules::RulesMatches;
use crate::server::TypstlabServer;
use rmcp::{ErrorData as McpError, model::*};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;
use typstlab_core::config::consts::search::{MAX_MATCHES, MAX_MATCHES_PER_FILE, MAX_SCAN_FILES};

#[derive(Debug, serde::Serialize)]
struct ExtendedSearchResult {
    query: RulesSearchArgs,
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
        serde_json::to_string(&outcome).map_err(errors::from_display)?,
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
        let mut current_offset = if pure_query.page < 1 {
            0
        } else {
            (pure_query.page - 1) * MAX_MATCHES
        };

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
                break;
            }
            let matches_quota = MAX_MATCHES - total_matches.len();
            let mut sub_config = config.clone();
            sub_config.max_matches = matches_quota;
            // Global offset handling:
            // We need to skip 'global_offset' matches across directories.
            // But we don't fully know how many matches correspond to 'global_offset' until we count them.
            // If we assume sequential search order is deterministic (sort_by_file_name), it is stable.
            // We pass 'global_offset' to search_dir_sync?
            // If search_dir_sync returns 'total_found' (before skipping), we can decrement global_offset.
            // But search_dir_sync currently doesn't return total_found.
            // I need to update SearchResult in common/types.rs first.
            // For now, I will modify the plan to update types.rs first.
            // I will return the original content here as this step is aborted to fix types.
            let mut sub_config = config.clone();
            sub_config.max_matches = matches_quota;

            // Apply current global offset to this directory search
            sub_config.offset = current_offset;

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
            let total_found_local = res.total_found;
            // Aggregate results
            total_matches.extend(res.matches);
            total_scanned += scanned_files;

            // Adjust offset for next directory:
            // We conceptually skipped 'total_found_local' items if we didn't take any.
            // If we took some, 'current_offset' was fully consumed in this dir (down to 0 effectively for next search, but next search shouldn't run if we are full).
            // Actually:
            // If current_offset was 50, and total_found was 30. We collected 0. We need to skip 20 more in next dir.
            // new_offset = current_offset - total_found = 20.
            // If current_offset was 50, and total_found was 60. We collected 10 (limit 50).
            // We are full (or up to provided limit).
            // Actually inner logic does: skip 'offset', take 'max_matches'.
            // So if we collected any matches, it means we passed the offset boundary.
            // So for next directory, offset should be 0 (we already started collecting).
            // BUT wait, we might have partial page fill.
            // If page size 50. Offset 0.
            // Dir1 has 10. We take 10. matches.len = 10.
            // Next loop: matches_quota = 40.
            // We need to fetch 40 from Dir2 starting at offset 0.
            // So logic implies:
            // If we collected > 0 matches, then for next dir offset is 0.
            // If we collected 0 matches, check if we "skipped" everything in this dir.
            // If total_found > offset: implies we should have collected something? Yes.
            // If matches is empty, it means total_found <= offset.
            // So we reduce offset by total_found.

            if !matches_empty {
                // We started collecting, so offset for subsequent dirs is 0
                current_offset = 0;
            } else {
                // Nothing collected from this dir.
                // We reduce the offset by what we found (and skipped).
                current_offset = current_offset.saturating_sub(total_found_local);
            }

            if truncated_local {
                // DESIGN.md 5.10.9: Do not clear results on scan limit.
                // We stop global search if one dir hits limits (scan or matches).
                // However, for scan limit, we might want to continue to next dir?
                // Spec says "return truncated=true if MAX_SCAN_FILES exceeded".
                // Logic in search_dir_sync sets truncated=true only if it stops early.
                // If we hit scan limit in Dir1, we should probably stop and return truncated=true to avoid huge scan?
                // Yes, limit applies globally roughly. But implementation applies per-dir scan limit.
                // Let's stick to simple logic: if any dir reports truncated, we stop.
                truncated = true;
                break;
            }
        }

        Ok(ExtendedSearchResult {
            query: pure_query,
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
