//! Rules search機能

use super::types::{RulesSearchArgs, collect_search_dirs};
use crate::errors;
use crate::server::TypstlabServer;
use rmcp::{ErrorData as McpError, model::*};
use serde_json::json;
use std::path::Path;
use tokio::fs;
use typstlab_core::config::consts::search::{
    MAX_FILE_BYTES, MAX_MATCHES, MAX_MATCHES_PER_FILE, MAX_SCAN_FILES,
};
use walkdir::WalkDir;

/// rules_searchハンドラ
pub(crate) async fn rules_search(
    server: &TypstlabServer,
    args: RulesSearchArgs,
) -> Result<CallToolResult, McpError> {
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

    let canonical_root = fs::canonicalize(&server.context.project_root)
        .await
        .map_err(errors::from_display)?;
    let search_dirs = collect_search_dirs(&server.context.project_root, &args);
    let query_lowercase = query.to_lowercase();
    let outcome = search_rules_dirs(
        &server.context.project_root,
        &canonical_root,
        &query_lowercase,
        search_dirs,
    )
    .await?;

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string(&json!({
            "matches": outcome.matches,
            "truncated": outcome.truncated,
        }))
        .map_err(errors::from_display)?,
    )]))
}

struct RulesSearchOutcome {
    matches: Vec<serde_json::Value>,
    truncated: bool,
}

struct SearchState<'a> {
    matches: &'a mut Vec<serde_json::Value>,
    scanned: &'a mut usize,
    truncated: &'a mut bool,
}

async fn search_rules_dirs(
    project_root: &Path,
    canonical_root: &Path,
    query: &str,
    search_dirs: Vec<(std::path::PathBuf, &'static str)>,
) -> Result<RulesSearchOutcome, McpError> {
    let mut matches = Vec::new();
    let mut truncated = false;
    let mut scanned = 0usize;
    for (dir, origin) in search_dirs {
        if !dir.exists() {
            continue;
        }
        let mut state = SearchState {
            matches: &mut matches,
            scanned: &mut scanned,
            truncated: &mut truncated,
        };
        if search_rules_dir(
            project_root,
            canonical_root,
            query,
            &dir,
            origin,
            &mut state,
        )
        .await?
        {
            break;
        }
        if truncated {
            break;
        }
    }
    Ok(RulesSearchOutcome { matches, truncated })
}

async fn search_rules_dir(
    project_root: &Path,
    canonical_root: &Path,
    query: &str,
    dir: &Path,
    origin: &str,
    state: &mut SearchState<'_>,
) -> Result<bool, McpError> {
    for entry in WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = match search_entry_path(&entry, canonical_root).await? {
            Some(path) => path,
            None => continue,
        };
        *state.scanned += 1;
        if *state.scanned > MAX_SCAN_FILES {
            *state.truncated = true;
            state.matches.clear();
            return Ok(true);
        }
        let file_matches = rules_matches_for_file(&path, project_root, query, origin).await?;
        state.matches.extend(file_matches);
        if state.matches.len() >= MAX_MATCHES {
            state.matches.truncate(MAX_MATCHES);
            return Ok(true);
        }
    }
    Ok(false)
}

async fn search_entry_path(
    entry: &walkdir::DirEntry,
    canonical_root: &Path,
) -> Result<Option<std::path::PathBuf>, McpError> {
    let file_type = entry.file_type();
    if file_type.is_symlink() || !file_type.is_file() {
        return Ok(None);
    }

    let path = entry.path();
    if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
        return Ok(None);
    }

    let canonical_target = match fs::canonicalize(path).await {
        Ok(target) => target,
        Err(_) => return Ok(None),
    };
    if !canonical_target.starts_with(canonical_root) {
        return Ok(None);
    }

    let metadata = fs::metadata(path).await.map_err(errors::from_display)?;
    if metadata.len() > MAX_FILE_BYTES {
        return Ok(None);
    }

    Ok(Some(path.to_path_buf()))
}

async fn rules_matches_for_file(
    path: &Path,
    project_root: &Path,
    query: &str,
    origin: &str,
) -> Result<Vec<serde_json::Value>, McpError> {
    let content = fs::read_to_string(path)
        .await
        .map_err(errors::from_display)?;
    let lines: Vec<&str> = content.lines().collect();
    let mut matches = Vec::new();

    for (line_index, line) in lines.iter().enumerate() {
        if line.to_lowercase().contains(query) {
            let start = line_index.saturating_sub(2);
            let end = (line_index + 2).min(lines.len().saturating_sub(1));
            let excerpt = lines[start..=end].join("\n");
            let rel_path = path
                .strip_prefix(project_root)
                .map_err(errors::from_display)?
                .to_string_lossy();
            matches.push(json!({
                "path": rel_path,
                "line": line_index + 1,
                "excerpt": excerpt,
                "origin": origin,
            }));
            if matches.len() >= MAX_MATCHES_PER_FILE {
                break;
            }
        }
    }

    Ok(matches)
}
