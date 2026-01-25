use crate::errors;
use crate::handlers::common::types::{SearchConfig, SearchResult};
use std::path::Path;
use tokio_util::sync::CancellationToken;
use walkdir::WalkDir;

use super::safety::check_entry_safety;

/// Search recursively synchronously (blocking).
pub fn search_dir_sync<F>(
    root: &Path,
    project_root: &Path, // Added project_root
    config: &SearchConfig,
    token: CancellationToken,
    mapper: F,
) -> Result<SearchResult, rmcp::ErrorData>
where
    F: Fn(&Path, &str) -> Option<Vec<serde_json::Value>>,
{
    let mut matches = Vec::new();
    let mut truncated = false;
    let mut scanned = 0usize;

    if !root.exists() {
        return Ok(SearchResult {
            matches: vec![],
            truncated: false,
            scanned_files: 0,
        });
    }

    check_entry_safety(root, project_root)?;

    for entry in WalkDir::new(root).follow_links(false).into_iter() {
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
            // Keep matches invalidation behavior?
            // Previous behavior: `matches.clear()` if truncated by file limit.
            // But if we hit limit, we might want to return what we found so far + truncated=true?
            // "DESIGN.md 5.10.9: Check limits before processing"
            // And previous code did `matches.clear(); break;`. Return empty if file limit hit.
            // Let's preserve that behavior for consistency.
            matches.clear();
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

        if let Some(file_matches) = mapper(path, &content) {
            matches.extend(file_matches);
            if matches.len() >= config.max_matches {
                matches.truncate(config.max_matches);
                truncated = true;
                break;
            }
        }
    }

    Ok(SearchResult {
        matches,
        truncated,
        scanned_files: scanned,
    })
}
