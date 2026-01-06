use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use typstlab_core::error::{Result as CoreResult, TypstlabError};
use walkdir::WalkDir;

// ============================================================================
// Input/Output Types
// ============================================================================

/// Scope for rules operations
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RulesScope {
    Project,
    Paper,
    All,
}

/// Subdirectory within rules/
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RulesSubdir {
    Paper,
    Scripts,
    Data,
    Misc,
}

// ----------------------------------------------------------------------------
// rules_list
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RulesListInput {
    pub scope: RulesScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdir: Option<RulesSubdir>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(default = "default_list_limit")]
    pub limit: usize,
}

fn default_list_limit() -> usize {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RulesListOutput {
    pub files: Vec<FileEntry>,
    pub total: usize,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
}

// ----------------------------------------------------------------------------
// rules_get
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RulesGetInput {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RulesGetOutput {
    pub path: String,
    pub content: String,
    pub size: u64,
    pub lines: usize,
    pub modified: DateTime<Utc>,
    pub sha256: Option<String>, // always null in v0.1
}

// ----------------------------------------------------------------------------
// rules_page
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RulesPageInput {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(default = "default_page_max_lines")]
    pub max_lines: usize,
}

fn default_page_max_lines() -> usize {
    200
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RulesPageOutput {
    pub path: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub total_lines: usize,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

// ----------------------------------------------------------------------------
// rules_search
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RulesSearchInput {
    pub query: String,
    pub scope: RulesScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper_id: Option<String>,
    #[serde(default = "default_search_limit")]
    pub limit: usize,
}

fn default_search_limit() -> usize {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RulesSearchOutput {
    pub matches: Vec<SearchMatch>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchMatch {
    pub path: String,
    pub line: usize,
    pub excerpt: String,
    pub context_before: String,
    pub context_after: String,
}

// ============================================================================
// Core Logic - Path Validation
// ============================================================================

/// Validate that a path is within project root and resolves safely
fn validate_rules_path(project_root: &Path, requested_path: &str) -> CoreResult<PathBuf> {
    let requested = Path::new(requested_path);

    // Check for absolute paths
    if requested.is_absolute() {
        return Err(TypstlabError::ProjectPathEscape {
            path: requested.to_path_buf(),
        });
    }

    // Check for .. in path
    if requested_path.contains("..") {
        return Err(TypstlabError::ProjectPathEscape {
            path: requested.to_path_buf(),
        });
    }

    // Resolve relative to project root
    let full_path = project_root.join(requested);

    // Check if file exists (canonicalize requires existence)
    if !full_path.exists() {
        return Err(TypstlabError::Generic(format!(
            "File not found: {}",
            requested_path
        )));
    }

    // Canonicalize to resolve symlinks
    let canonical = full_path.canonicalize().map_err(|e| {
        TypstlabError::Generic(format!("Failed to resolve path: {}", e))
    })?;

    // Verify it's still within project root
    if !canonical.starts_with(project_root) {
        return Err(TypstlabError::ProjectPathEscape {
            path: requested.to_path_buf(),
        });
    }

    Ok(canonical)
}

/// Resolve rules directory path based on scope and subdir
fn resolve_rules_dir(
    project_root: &Path,
    scope: &RulesScope,
    paper_id: Option<&str>,
    subdir: Option<&RulesSubdir>,
) -> CoreResult<PathBuf> {
    let base = match scope {
        RulesScope::Project => project_root.join("rules"),
        RulesScope::Paper => {
            let paper_id = paper_id.ok_or_else(|| {
                TypstlabError::Generic("paper_id required for scope=paper".to_string())
            })?;
            project_root.join("papers").join(paper_id).join("rules")
        }
        RulesScope::All => {
            return Err(TypstlabError::Generic(
                "scope=all not supported for directory resolution".to_string(),
            ));
        }
    };

    let path = if let Some(subdir) = subdir {
        let subdir_name = match subdir {
            RulesSubdir::Paper => "paper",
            RulesSubdir::Scripts => "scripts",
            RulesSubdir::Data => "data",
            RulesSubdir::Misc => "misc",
        };
        base.join(subdir_name)
    } else {
        base
    };

    Ok(path)
}

// ============================================================================
// Core Logic - Line-Based Paging
// ============================================================================

/// Read a range of lines from a file
/// Returns (content, actual_lines_read, total_lines, has_more)
fn read_lines_range(
    path: &Path,
    start_line: usize,
    max_lines: usize,
) -> CoreResult<(String, usize, usize, bool)> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut lines_vec: Vec<String> = Vec::new();
    let mut total_lines = 0;

    for line_result in reader.lines() {
        total_lines += 1;
        let line = line_result?;

        if total_lines >= start_line {
            lines_vec.push(line);
            if lines_vec.len() >= max_lines {
                break;
            }
        }
    }

    // If we stopped because we hit max_lines, check if there are more lines
    let has_more = if lines_vec.len() == max_lines {
        // Try to read one more line to see if there are more
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let mut line_count = 0;
        for _ in reader.lines() {
            line_count += 1;
            if line_count > start_line + max_lines - 1 {
                // There's at least one more line
                return Ok((
                    lines_vec.join("\n"),
                    lines_vec.len(),
                    line_count,
                    true,
                ));
            }
        }
        false
    } else {
        false
    };

    let content = lines_vec.join("\n");
    Ok((content, lines_vec.len(), total_lines, has_more))
}

// ============================================================================
// Tool Handlers
// ============================================================================

/// List files in rules/ directories with pagination
pub fn rules_list(input: RulesListInput, project_root: &Path) -> CoreResult<RulesListOutput> {
    // Validate limits
    if input.limit > 200 {
        return Err(TypstlabError::Generic(
            "limit cannot exceed 200".to_string(),
        ));
    }

    // Resolve directory
    let rules_dir = resolve_rules_dir(
        project_root,
        &input.scope,
        input.paper_id.as_deref(),
        input.subdir.as_ref(),
    )?;

    // Check if directory exists
    if !rules_dir.exists() {
        return Ok(RulesListOutput {
            files: vec![],
            total: 0,
            has_more: false,
            next_cursor: None,
        });
    }

    // Collect all .md files
    let mut all_files: Vec<FileEntry> = Vec::new();

    for entry in WalkDir::new(&rules_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip directories
        if !path.is_file() {
            continue;
        }

        // Only .md files
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }

        // Skip hidden files
        if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
            if filename.starts_with('.') {
                continue;
            }
        }

        // Get metadata
        let metadata = fs::metadata(path)?;
        let size = metadata.len();
        let modified = metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| DateTime::from_timestamp(d.as_secs() as i64, 0).unwrap_or_default())
            .unwrap_or_default();

        // Get relative path from project root
        let relative_path = path.strip_prefix(project_root).map_err(|e| {
            TypstlabError::Generic(format!("Path error: {}", e))
        })?;

        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| TypstlabError::Generic("Invalid filename".to_string()))?
            .to_string();

        all_files.push(FileEntry {
            name,
            path: relative_path.to_string_lossy().to_string(),
            size,
            modified,
        });
    }

    // Sort by path for stable ordering
    all_files.sort_by(|a, b| a.path.cmp(&b.path));

    let total = all_files.len();

    // Handle pagination with simple string cursor
    let start_index = if let Some(cursor_str) = &input.cursor {
        cursor_str
            .parse::<usize>()
            .map_err(|_| TypstlabError::Generic("Invalid cursor".to_string()))?
    } else {
        0
    };

    let end_index = std::cmp::min(start_index + input.limit, total);
    let files: Vec<FileEntry> = all_files[start_index..end_index].to_vec();

    let has_more = end_index < total;
    let next_cursor = if has_more {
        Some(end_index.to_string())
    } else {
        None
    };

    Ok(RulesListOutput {
        files,
        total,
        has_more,
        next_cursor,
    })
}

/// Get full content of a rules file
pub fn rules_get(input: RulesGetInput, project_root: &Path) -> CoreResult<RulesGetOutput> {
    const MAX_BYTES: u64 = 262144; // 256KB

    // Validate and resolve path
    let path = validate_rules_path(project_root, &input.path)?;

    // Check file size
    let metadata = fs::metadata(&path)?;
    let size = metadata.len();

    if size > MAX_BYTES {
        return Err(TypstlabError::Generic(format!(
            "File too large: {} bytes (max {})",
            size, MAX_BYTES
        )));
    }

    // Read content
    let content = fs::read_to_string(&path)?;

    // Count lines
    let lines = content.lines().count();

    // Get modified time
    let modified = metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| DateTime::from_timestamp(d.as_secs() as i64, 0).unwrap_or_default())
        .unwrap_or_default();

    Ok(RulesGetOutput {
        path: input.path,
        content,
        size,
        lines,
        modified,
        sha256: None, // always null in v0.1
    })
}

/// Get file content in line-based chunks
pub fn rules_page(input: RulesPageInput, project_root: &Path) -> CoreResult<RulesPageOutput> {
    // Validate max_lines
    if input.max_lines > 400 {
        return Err(TypstlabError::Generic(
            "max_lines cannot exceed 400".to_string(),
        ));
    }

    // Validate and resolve path
    let path = validate_rules_path(project_root, &input.path)?;

    // Determine start line with simple string cursor
    let start_line = if let Some(cursor_str) = &input.cursor {
        cursor_str
            .parse::<usize>()
            .map_err(|_| TypstlabError::Generic("Invalid cursor".to_string()))?
    } else {
        1 // Line numbers start at 1
    };

    // Read lines
    let (content, actual_lines, total_lines, has_more) =
        read_lines_range(&path, start_line, input.max_lines)?;

    let end_line = start_line + actual_lines - 1;

    let next_cursor = if has_more {
        Some((end_line + 1).to_string())
    } else {
        None
    };

    Ok(RulesPageOutput {
        path: input.path,
        content,
        start_line,
        end_line,
        total_lines,
        has_more,
        next_cursor,
    })
}

/// Search across all rules files
pub fn rules_search(input: RulesSearchInput, project_root: &Path) -> CoreResult<RulesSearchOutput> {
    // Validate limit
    if input.limit > 50 {
        return Err(TypstlabError::Generic(
            "limit cannot exceed 50".to_string(),
        ));
    }

    // Determine search directories
    let search_dirs: Vec<PathBuf> = match input.scope {
        RulesScope::Project => {
            vec![project_root.join("rules")]
        }
        RulesScope::Paper => {
            let paper_id = input.paper_id.as_ref().ok_or_else(|| {
                TypstlabError::Generic("paper_id required for scope=paper".to_string())
            })?;
            vec![project_root.join("papers").join(paper_id).join("rules")]
        }
        RulesScope::All => {
            let mut dirs = vec![project_root.join("rules")];
            // Add all paper rules directories
            let papers_dir = project_root.join("papers");
            if papers_dir.exists() {
                for entry in fs::read_dir(papers_dir)? {
                    let entry = entry?;
                    if entry.path().is_dir() {
                        dirs.push(entry.path().join("rules"));
                    }
                }
            }
            dirs
        }
    };

    let query_lower = input.query.to_lowercase();
    let mut matches: Vec<SearchMatch> = Vec::new();
    let mut match_count = 0;

    for search_dir in search_dirs {
        if !search_dir.exists() {
            continue;
        }

        for entry in WalkDir::new(&search_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Only .md files
            if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            // Skip hidden files
            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                if filename.starts_with('.') {
                    continue;
                }
            }

            // Search file content
            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let lines: Vec<&str> = content.lines().collect();
            let mut file_matches = 0;

            for (line_num, line) in lines.iter().enumerate() {
                if file_matches >= 3 {
                    break; // Max 3 matches per file
                }

                if match_count >= input.limit {
                    break;
                }

                if line.to_lowercase().contains(&query_lower) {
                    // Get context
                    let context_before = if line_num > 0 {
                        lines[line_num - 1].to_string()
                    } else {
                        String::new()
                    };

                    let context_after = if line_num + 1 < lines.len() {
                        lines[line_num + 1].to_string()
                    } else {
                        String::new()
                    };

                    // Get relative path
                    let relative_path = path.strip_prefix(project_root).map_err(|e| {
                        TypstlabError::Generic(format!("Path error: {}", e))
                    })?;

                    matches.push(SearchMatch {
                        path: relative_path.to_string_lossy().to_string(),
                        line: line_num + 1, // Line numbers start at 1
                        excerpt: (*line).to_string(),
                        context_before,
                        context_after,
                    });

                    file_matches += 1;
                    match_count += 1;
                }
            }

            if match_count >= input.limit {
                break;
            }
        }

        if match_count >= input.limit {
            break;
        }
    }

    Ok(RulesSearchOutput {
        total: matches.len(),
        matches,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_rules_path_rejects_absolute() {
        let project_root = PathBuf::from("/tmp/project");
        let result = validate_rules_path(&project_root, "/etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_rules_path_rejects_dotdot() {
        let project_root = PathBuf::from("/tmp/project");
        let result = validate_rules_path(&project_root, "../../../etc/passwd");
        assert!(result.is_err());
    }
}
