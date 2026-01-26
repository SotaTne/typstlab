use crate::errors;
use crate::handlers::common::types::{SearchConfig, SearchResult};
use std::path::Path;
use tokio_util::sync::CancellationToken;
use walkdir::WalkDir;

use super::safety::check_entry_safety;

/// Search recursively synchronously (blocking).
pub fn search_dir_sync<T, F, Q>(
    root: &Path,
    project_root: &Path, // Added project_root
    config: &SearchConfig,
    token: CancellationToken,
    mapper: F,
    query: Q,
) -> Result<SearchResult<T, Q>, rmcp::ErrorData>
where
    F: Fn(&Path, &str, &std::fs::Metadata) -> Option<Vec<T>>,
    Q: serde::Serialize,
    T: serde::Serialize,
{
    let mut matches = Vec::new();
    let mut truncated = false;
    let mut scanned = 0usize;
    let mut total_found = 0usize;

    if !root.exists() {
        return Ok(SearchResult {
            query,
            matches: vec![],
            truncated: false,
            scanned_files: 0,
            total_found: 0,
        });
    }

    check_entry_safety(root, project_root)?;

    for entry in WalkDir::new(root)
        .sort_by_file_name()
        .follow_links(false)
        .into_iter()
    {
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

        // Scan Limits: Count processed files
        if entry.file_type().is_dir() {
            continue;
        }

        // Safety check
        let path = entry.path();
        if check_entry_safety(path, project_root).is_err() {
            continue;
        }

        if let Some(ext) = path.extension() {
            if !config
                .file_extensions
                .contains(&ext.to_string_lossy().to_string())
            {
                continue;
            }
        } else {
            continue;
        }

        if scanned >= config.max_files {
            truncated = true;
            // DESIGN.md 5.10.9: Return empty matches if scan limit exceeded?
            // User requirement: "pageではtrunkで含んでいないその次からを返すようにします"
            // "trunk=trueの時に... pageを+1すれば検索できるようにします"
            // If we stop at SCAN limit, we might have partial matches.
            // We should return them. Previous logic cleared them.
            // The requirement says: do NOT clear results when truncated.
            // So we just break and return what we have.
            break;
        }

        scanned += 1;

        if token.is_cancelled() {
            return Err(errors::request_cancelled());
        }

        let metadata = std::fs::metadata(path).map_err(errors::from_display)?;
        if metadata.len() > typstlab_core::config::consts::search::MAX_FILE_BYTES {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if let Some(file_matches) = mapper(path, &content, &metadata) {
            for m in file_matches {
                total_found += 1;
                if total_found <= config.offset {
                    continue;
                }
                matches.push(m);
                if matches.len() > config.max_matches {
                    // We found one more than max_matches (offset + limit + 1)
                    // This indicates there are more results (next page exists).
                    matches.pop(); // Remove the extra one
                    truncated = true;
                    // We can break early here because we know there's more.
                    // However, we must ensure we break the OUTER loop.
                    // But wait, if we break, 'scanned' stops incrementing.
                    // Does it matter? Scanned is just a stat/limit check.
                    // If we found enough matches, we don't need to scan more files.
                    // So yes, we can break.
                    break;
                }
            }
            if truncated {
                break;
            }
        }
    }

    Ok(SearchResult {
        query,
        matches,
        truncated,
        scanned_files: scanned,
        total_found,
    })
}
