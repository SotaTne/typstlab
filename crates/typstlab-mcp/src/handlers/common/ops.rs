use std::path::Path;
use tokio::fs;
use typstlab_core::path::has_absolute_or_rooted_component;

use super::types::{BrowseItem, BrowseResult, SearchConfig, SearchMatch, SearchResult};

/// Browse a directory and return its contents
///
/// # Arguments
/// * `root` - The root directory to browse from
/// * `relative_path` - Optional relative path within the root
/// * `file_extensions` - File extensions to include (e.g., ["md", "typ"])
///
/// # Returns
/// A BrowseResult containing the directory listing or indicating if the directory is missing
pub async fn browse_directory(
    root: &Path,
    relative_path: Option<&str>,
    file_extensions: &[String],
) -> Result<BrowseResult, rmcp::ErrorData> {
    let target_dir = if let Some(rel_path) = relative_path {
        let rel = Path::new(rel_path);

        // Validate path
        if has_absolute_or_rooted_component(rel) {
            return Err(rmcp::ErrorData::invalid_params(
                "Path cannot be absolute or rooted",
                None,
            ));
        }

        if rel.components().any(|c| c.as_os_str() == "..") {
            return Err(rmcp::ErrorData::invalid_params(
                "Path cannot contain ..",
                None,
            ));
        }

        root.join(rel)
    } else {
        root.to_path_buf()
    };

    // Check if directory exists
    if !target_dir.exists() {
        return Ok(BrowseResult {
            missing: true,
            items: vec![],
        });
    }

    let mut items = Vec::new();
    let mut entries = fs::read_dir(&target_dir).await.map_err(|e| {
        rmcp::ErrorData::internal_error(format!("Failed to read directory: {}", e), None)
    })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| {
        rmcp::ErrorData::internal_error(format!("Failed to read entry: {}", e), None)
    })? {
        let path = entry.path();
        let metadata = entry.metadata().await.map_err(|e| {
            rmcp::ErrorData::internal_error(format!("Failed to read metadata: {}", e), None)
        })?;

        // Skip symlinks that point outside the root
        if metadata.is_symlink()
            && let Ok(canonical) = fs::canonicalize(&path).await
            && let Ok(root_canonical) = fs::canonicalize(root).await
            && !canonical.starts_with(&root_canonical)
        {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let item_type = if metadata.is_dir() {
            "directory"
        } else {
            // Check file extension
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_string();
                if !file_extensions.contains(&ext_str) {
                    continue;
                }
            } else {
                continue;
            }
            "file"
        };

        items.push(BrowseItem {
            name,
            item_type: item_type.to_string(),
        });
    }

    Ok(BrowseResult {
        missing: false,
        items,
    })
}

/// Search for a query string in files within a directory
///
/// # Arguments
/// * `root` - The root directory to search in
/// * `query` - The search query (case-insensitive)
/// * `config` - Search configuration (limits, file types, etc.)
///
/// # Returns
/// A SearchResult containing matches and truncation status
pub async fn search_directory(
    root: &Path,
    query: &str,
    config: &SearchConfig,
) -> Result<SearchResult, rmcp::ErrorData> {
    let query_lower = query.to_lowercase();
    let mut matches = Vec::new();
    let mut files_scanned = 0;
    let mut truncated = false;

    if !root.exists() {
        return Ok(SearchResult {
            matches: vec![],
            truncated: false,
        });
    }

    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        if files_scanned >= config.max_files {
            truncated = true;
            matches.clear();
            break;
        }

        let mut entries = match fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(_) => continue,
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let metadata = match entry.metadata().await {
                Ok(m) => m,
                Err(_) => continue,
            };

            // Skip symlinks that point outside the root
            if metadata.is_symlink()
                && let Ok(canonical) = fs::canonicalize(&path).await
                && let Ok(root_canonical) = fs::canonicalize(root).await
                && !canonical.starts_with(&root_canonical)
            {
                continue;
            }

            if metadata.is_dir() {
                stack.push(path);
            } else if metadata.is_file() {
                // Check file extension
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_string();
                    if !config.file_extensions.contains(&ext_str) {
                        continue;
                    }
                } else {
                    continue;
                }

                files_scanned += 1;
                if files_scanned > config.max_files {
                    truncated = true;
                    matches.clear();
                    break;
                }

                // Read and search file
                if let Ok(content) = fs::read_to_string(&path).await {
                    for (line_num, line) in content.lines().enumerate() {
                        if line.to_lowercase().contains(&query_lower) {
                            let relative_path = path
                                .strip_prefix(root)
                                .unwrap_or(&path)
                                .to_string_lossy()
                                .to_string();

                            matches.push(SearchMatch {
                                path: relative_path,
                                line: line_num + 1,
                                content: line.to_string(),
                            });

                            if matches.len() >= config.max_matches {
                                return Ok(SearchResult {
                                    matches,
                                    truncated: false,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(SearchResult { matches, truncated })
}
