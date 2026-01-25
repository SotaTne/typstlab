use crate::errors;
use crate::handlers::common::types::{BrowseItem, BrowseResult, SearchConfig, SearchResult};
use std::path::Path;
use tokio_util::sync::CancellationToken;
use typstlab_core::path::has_absolute_or_rooted_component;
use walkdir::WalkDir;

/// Check if a path entry is safe to process.
/// `path`: The path to check
/// `project_root`: The root of the project to ensure no escape
pub fn check_entry_safety(path: &Path, project_root: &Path) -> Result<(), rmcp::ErrorData> {
    // 1. Symlink check (Strictly forbidden)
    // Note: If path is project_root/rules (symlink), we should probably allow it IF it points inside project?
    // User requirement: "rules/docs のルート自体がシンボリックリンクでも許容される... プロジェクト外へのエスケープを完全に防げていない"
    // So strictly forbid symlinks even for roots if they point outside?
    // Actually typically symlinks are forbidden entirely in this project design to avoid complexity.
    let metadata = std::fs::symlink_metadata(path).map_err(|e| {
        rmcp::ErrorData::internal_error(format!("Failed to read metadata: {}", e), None)
    })?;

    if metadata.is_symlink() {
        return Err(errors::path_escape("Symlinks are not allowed"));
    }

    // 2. Canonicalize check (Path escape)
    let canonical_path = std::fs::canonicalize(path).map_err(|e| {
        // If file doesn't exist, canonicalize fails. Check existence before?
        // Caller usually ensures existence or we handle error.
        rmcp::ErrorData::internal_error(format!("Canonicalize failed: {}", e), None)
    })?;

    let canonical_root = std::fs::canonicalize(project_root).map_err(|e| {
        rmcp::ErrorData::internal_error(format!("Canonicalize project root failed: {}", e), None)
    })?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err(errors::path_escape("Path escapes project root"));
    }

    Ok(())
}

/// Browse a directory synchronously (blocking).
pub fn browse_dir_sync(
    root: &Path,
    project_root: &Path, // Added project_root
    relative_path: Option<&str>,
    file_extensions: &[String],
    limit: usize, // Added limit
    token: CancellationToken,
) -> Result<BrowseResult, rmcp::ErrorData> {
    let target_dir = if let Some(rel_path) = relative_path {
        let rel = Path::new(rel_path);
        if has_absolute_or_rooted_component(rel) {
            return Err(errors::path_escape("Path cannot be absolute or rooted"));
        }
        if rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(errors::path_escape("Path cannot contain .."));
        }
        root.join(rel)
    } else {
        root.to_path_buf()
    };

    if !target_dir.exists() || !target_dir.is_dir() {
        return Ok(BrowseResult {
            missing: true,
            items: vec![],
            truncated: false,
        });
    }

    // Check safety of the target directory itself against PROJECT root
    check_entry_safety(&target_dir, project_root)?;

    let mut items = Vec::new();
    let mut truncated = false;

    let entries = std::fs::read_dir(&target_dir).map_err(errors::from_display)?;

    for entry in entries {
        if token.is_cancelled() {
            return Err(errors::request_cancelled());
        }

        if items.len() >= limit {
            truncated = true;
            break;
        }

        let entry = entry.map_err(errors::from_display)?;
        let path = entry.path();

        if let Err(e) = check_entry_safety(&path, project_root) {
            tracing::warn!("Skipping unsafe entry {:?}: {:?}", path, e);
            continue;
        }

        let metadata = entry.metadata().map_err(errors::from_display)?;
        let item_type = if metadata.is_dir() {
            "directory"
        } else {
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

        let relative_path = path
            .strip_prefix(project_root)
            .map(|p| {
                // Convert to forward slashes for cross-platform consistency (AGENTS.md)
                p.to_string_lossy().replace('\\', "/")
            })
            .unwrap_or_else(|_| entry.file_name().to_string_lossy().to_string());

        items.push(BrowseItem {
            name: entry.file_name().to_string_lossy().to_string(),
            path: relative_path,
            item_type: item_type.to_string(),
        });
    }

    items.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(BrowseResult {
        missing: false,
        items,
        truncated,
    })
}

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
