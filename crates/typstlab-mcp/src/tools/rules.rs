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
///
/// Security: Prevents path traversal attacks by:
/// 1. Blocking absolute paths
/// 2. Blocking ".." directory traversal using Path::components()
/// 3. Requiring "rules/" or "papers/<paper_id>/rules/" prefix
/// 4. Blocking empty paper_id in papers/ paths
/// 5. Canonicalizing to resolve symlinks (when file exists)
/// 6. Verifying final path is within project root
///
/// Cross-platform: Uses Path::components() to support both / and \ separators
fn validate_rules_path(
    project_root: &Path,
    requested_path: impl AsRef<Path>,
) -> CoreResult<PathBuf> {
    use std::path::Component;

    let requested = requested_path.as_ref();

    // 1. Check for absolute paths
    if requested.is_absolute() {
        return Err(TypstlabError::ProjectPathEscape {
            path: requested.to_path_buf(),
        });
    }

    // 2. Normalize components (drop leading "./") and check for parent traversal
    let mut components: Vec<Component> = requested.components().collect();
    while components.first() == Some(&Component::CurDir) {
        components.remove(0);
    }
    if components.iter().any(|c| matches!(c, Component::ParentDir)) {
        return Err(TypstlabError::ProjectPathEscape {
            path: requested.to_path_buf(),
        });
    }

    // 3. Must start with "rules/" or "papers/<paper_id>/rules/"
    let allowed_base = if components.first() == Some(&Component::Normal("rules".as_ref())) {
        "rules"
    } else if components.first() == Some(&Component::Normal("papers".as_ref())) {
        // papers/ の場合、papers/<paper_id>/rules/ パターンを検証
        if components.len() < 4 {
            return Err(TypstlabError::Generic(format!(
                "Path must be within rules/ or papers/<paper_id>/rules/ directory, got: {}",
                requested.display()
            )));
        }

        // components[1] must be non-empty paper_id
        match components.get(1) {
            Some(Component::Normal(name)) => {
                let name_str = name
                    .to_str()
                    .ok_or_else(|| TypstlabError::Generic("Invalid UTF-8 in path".to_string()))?;
                if name_str.is_empty() {
                    return Err(TypstlabError::Generic(format!(
                        "paper_id cannot be empty, got: {}",
                        requested.display()
                    )));
                }
            }
            _ => {
                return Err(TypstlabError::Generic(format!(
                    "Invalid paper_id component, got: {}",
                    requested.display()
                )));
            }
        }

        // components[2] must be "rules"
        if components.get(2) != Some(&Component::Normal("rules".as_ref())) {
            return Err(TypstlabError::Generic(format!(
                "Path must be within rules/ or papers/<paper_id>/rules/ directory, got: {}",
                requested.display()
            )));
        }

        "papers"
    } else {
        return Err(TypstlabError::Generic(format!(
            "Path must be within rules/ or papers/<paper_id>/rules/ directory, got: {}",
            requested.display()
        )));
    };

    // 4. Construct full path and canonicalize
    let full_path = project_root.join(requested);

    // 5. If file exists, verify it resolves within project root AND allowed directory
    if full_path.exists() {
        let canonical = full_path
            .canonicalize()
            .map_err(|e| TypstlabError::Generic(format!("Path canonicalization failed: {}", e)))?;

        let canonical_root = project_root
            .canonicalize()
            .map_err(|e| TypstlabError::Generic(format!("Root canonicalization failed: {}", e)))?;

        // Check if canonical path is within project root
        if !canonical.starts_with(&canonical_root) {
            return Err(TypstlabError::ProjectPathEscape { path: canonical });
        }

        // Check if canonical path is within the allowed base directory (rules/ or papers/)
        let allowed_base_full = canonical_root.join(allowed_base);
        if !canonical.starts_with(&allowed_base_full) {
            return Err(TypstlabError::ProjectPathEscape { path: canonical });
        }

        // Additional check for papers/: must be within papers/<paper_id>/rules/
        if allowed_base == "papers" {
            // paper_id already validated above
            let mut components: Vec<Component> = requested.components().collect();
            while components.first() == Some(&Component::CurDir) {
                components.remove(0);
            }
            if let Some(Component::Normal(paper_id)) = components.get(1) {
                let paper_id_str = paper_id
                    .to_str()
                    .ok_or_else(|| TypstlabError::Generic("Invalid UTF-8 in path".to_string()))?;
                let expected_base = canonical_root
                    .join("papers")
                    .join(paper_id_str)
                    .join("rules");
                if !canonical.starts_with(&expected_base) {
                    return Err(TypstlabError::ProjectPathEscape { path: canonical });
                }
            }
        }

        Ok(canonical)
    } else {
        // File doesn't exist, but path structure is valid
        Ok(full_path)
    }
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
    let mut line_count = 0;
    let mut has_more = false;

    for line_result in reader.lines() {
        line_count += 1;
        let line = line_result?;

        if line_count >= start_line && lines_vec.len() < max_lines {
            lines_vec.push(line);
        } else if lines_vec.len() >= max_lines {
            // We've collected enough lines, but continue counting
            has_more = true;
            // Continue to count total lines
        }
    }

    let content = lines_vec.join("\n");
    Ok((content, lines_vec.len(), line_count, has_more))
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
        if let Some(filename) = path.file_name().and_then(|s| s.to_str())
            && filename.starts_with('.')
        {
            continue;
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
        let relative_path = path
            .strip_prefix(project_root)
            .map_err(|e| TypstlabError::Generic(format!("Path error: {}", e)))?;

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

    // Validate offset
    if start_index > total {
        return Err(TypstlabError::Generic(format!(
            "Offset {} exceeds total files {}",
            start_index, total
        )));
    }

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
    let requested_path = Path::new(&input.path);
    let path = validate_rules_path(project_root, requested_path)?;

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
    if input.max_lines == 0 {
        return Err(TypstlabError::Generic(
            "max_lines must be at least 1".to_string(),
        ));
    }
    if input.max_lines > 400 {
        return Err(TypstlabError::Generic(
            "max_lines cannot exceed 400".to_string(),
        ));
    }

    // Validate and resolve path
    let requested_path = Path::new(&input.path);
    let path = validate_rules_path(project_root, requested_path)?;

    // Determine start line with simple string cursor
    let start_line = if let Some(cursor_str) = &input.cursor {
        let line = cursor_str
            .parse::<usize>()
            .map_err(|_| TypstlabError::Generic("Invalid cursor".to_string()))?;

        // Line numbers must be >= 1 (1-indexed)
        if line == 0 {
            return Err(TypstlabError::Generic(
                "Invalid cursor: line numbers start at 1".to_string(),
            ));
        }

        line
    } else {
        1 // Line numbers start at 1
    };

    // Read lines
    let (content, actual_lines, total_lines, has_more) =
        read_lines_range(&path, start_line, input.max_lines)?;

    // Handle empty file: return start_line=1, end_line=0 (1-indexed with empty range)
    // Empty range is represented by end_line < start_line (documented API behavior)
    // This maintains: end_line <= total_lines (0 <= 0) and 1-indexed consistency
    // Allow cursor=None or cursor=1, reject cursor >= 2
    if total_lines == 0 {
        if let Some(cursor_str) = &input.cursor {
            let cursor_line = cursor_str
                .parse::<usize>()
                .map_err(|_| TypstlabError::Generic("Invalid cursor".to_string()))?;

            if cursor_line > 1 {
                return Err(TypstlabError::Generic(format!(
                    "Empty file (total_lines=0): cursor must be 1 if provided, got {}",
                    cursor_line
                )));
            }
        }

        return Ok(RulesPageOutput {
            path: input.path,
            content: String::new(),
            start_line: 1, // Always 1-indexed (no exceptions)
            end_line: 0,   // Empty range: end_line < start_line
            total_lines: 0,
            has_more: false,
            next_cursor: None,
        });
    }

    // Validate cursor for non-empty files
    if start_line > total_lines {
        return Err(TypstlabError::Generic(format!(
            "Start line {} exceeds total lines {}",
            start_line, total_lines
        )));
    }

    // Calculate end_line for non-empty files
    let end_line = if actual_lines > 0 {
        start_line + actual_lines - 1
    } else {
        // Non-empty file but no lines read (should not happen after validation)
        start_line
    };

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
        return Err(TypstlabError::Generic("limit cannot exceed 50".to_string()));
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
            if let Some(filename) = path.file_name().and_then(|s| s.to_str())
                && filename.starts_with('.')
            {
                continue;
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
                    let relative_path = path
                        .strip_prefix(project_root)
                        .map_err(|e| TypstlabError::Generic(format!("Path error: {}", e)))?;

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
        let result = validate_rules_path(&project_root, Path::new("/etc/passwd"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_rules_path_rejects_dotdot() {
        let project_root = PathBuf::from("/tmp/project");
        let result = validate_rules_path(&project_root, Path::new("../../../etc/passwd"));
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;
    use std::fs;
    use typstlab_testkit::temp_dir_in_workspace;

    #[test]
    fn test_path_traversal_blocked() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        // Create a secret file outside rules/
        let secret_file = project_root.join(".env");
        fs::write(&secret_file, "SECRET=password123").unwrap();

        // Attempt 1: Direct parent traversal
        let result = validate_rules_path(project_root, Path::new("../.env"));
        assert!(result.is_err(), "Should block direct parent traversal");

        // Attempt 2: Deep parent traversal
        let result = validate_rules_path(project_root, Path::new("rules/../../.env"));
        assert!(result.is_err(), "Should block deep parent traversal");

        // Attempt 3: Absolute path
        let result = validate_rules_path(project_root, &secret_file);
        assert!(result.is_err(), "Should block absolute path");

        // Attempt 4: Non-rules path
        let result = validate_rules_path(project_root, Path::new("src/main.rs"));
        assert!(result.is_err(), "Should block non-rules/papers path");
    }

    #[test]
    fn test_valid_rules_paths() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        // Create rules directory
        fs::create_dir_all(project_root.join("rules/paper")).unwrap();
        fs::write(project_root.join("rules/test.md"), "content").unwrap();

        // Valid path - existing file
        let result = validate_rules_path(project_root, Path::new("rules/test.md"));
        assert!(result.is_ok(), "Should allow valid existing file");

        // Valid subdirectory path - non-existing file
        let result = validate_rules_path(project_root, Path::new("rules/paper/guide.md"));
        assert!(result.is_ok(), "Should allow valid non-existing file");
    }

    #[test]
    fn test_papers_directory_allowed() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        // Create papers directory
        fs::create_dir_all(project_root.join("papers/paper1/rules")).unwrap();
        fs::write(project_root.join("papers/paper1/rules/guide.md"), "content").unwrap();

        // Valid papers path
        let result = validate_rules_path(project_root, Path::new("papers/paper1/rules/guide.md"));
        assert!(result.is_ok(), "Should allow papers/ prefix");
    }

    #[test]
    #[cfg(unix)]
    fn test_symlink_escape_blocked() {
        use std::os::unix::fs::symlink;

        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        // Create rules directory and secret file
        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join(".env"), "SECRET=123").unwrap();

        // Create symlink pointing outside rules/
        let symlink_path = project_root.join("rules/escape.md");
        symlink(project_root.join(".env"), &symlink_path).unwrap();

        // Should be blocked when canonicalized
        let result = validate_rules_path(project_root, Path::new("rules/escape.md"));
        assert!(
            result.is_err(),
            "Should block symlink escaping project root"
        );
    }

    #[test]
    fn test_rules_get_with_path_validation() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        // Create valid file
        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join("rules/test.md"), "test content").unwrap();

        // Valid request
        let input = RulesGetInput {
            path: "rules/test.md".to_string(),
        };
        let result = rules_get(input, project_root);
        assert!(result.is_ok(), "Should allow valid path");

        // Invalid request - path traversal
        let input = RulesGetInput {
            path: "../.env".to_string(),
        };
        let result = rules_get(input, project_root);
        assert!(result.is_err(), "Should block path traversal in rules_get");
    }

    #[test]
    fn test_rules_page_with_path_validation() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        // Create valid file
        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join("rules/test.md"), "line1\nline2\nline3").unwrap();

        // Valid request
        let input = RulesPageInput {
            path: "rules/test.md".to_string(),
            cursor: None,
            max_lines: 10,
        };
        let result = rules_page(input, project_root);
        assert!(result.is_ok(), "Should allow valid path");

        // Invalid request - path traversal
        let input = RulesPageInput {
            path: "../.env".to_string(),
            cursor: None,
            max_lines: 10,
        };
        let result = rules_page(input, project_root);
        assert!(result.is_err(), "Should block path traversal in rules_page");
    }
}

#[cfg(test)]
mod security_tests_v2 {
    use super::*;
    use std::fs;
    use typstlab_testkit::temp_dir_in_workspace;

    #[test]
    fn test_papers_direct_access_blocked() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        // Create papers directory with files
        fs::create_dir_all(project_root.join("papers/paper1/rules")).unwrap();
        fs::write(project_root.join("papers/paper1/main.typ"), "content").unwrap();
        fs::write(project_root.join("papers/paper1/paper.toml"), "content").unwrap();
        fs::write(project_root.join("papers/paper1/rules/guide.md"), "content").unwrap();

        // Should block: papers/<paper_id>/main.typ
        let result = validate_rules_path(project_root, Path::new("papers/paper1/main.typ"));
        assert!(result.is_err(), "Should block papers/paper1/main.typ");

        // Should block: papers/<paper_id>/paper.toml
        let result = validate_rules_path(project_root, Path::new("papers/paper1/paper.toml"));
        assert!(result.is_err(), "Should block papers/paper1/paper.toml");

        // Should allow: papers/<paper_id>/rules/guide.md
        let result = validate_rules_path(project_root, Path::new("papers/paper1/rules/guide.md"));
        assert!(result.is_ok(), "Should allow papers/paper1/rules/guide.md");
    }

    #[test]
    fn test_papers_shallow_path_blocked() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        // Should block: papers/ (too shallow)
        let result = validate_rules_path(project_root, Path::new("papers/test.md"));
        assert!(result.is_err(), "Should block papers/test.md (no paper_id)");

        // Should block: papers/<paper_id>/ (no rules/)
        let result = validate_rules_path(project_root, Path::new("papers/paper1/test.md"));
        assert!(
            result.is_err(),
            "Should block papers/paper1/test.md (no rules/)"
        );
    }
}

#[cfg(test)]
mod bounds_tests_v2 {
    use super::*;
    use std::fs;
    use typstlab_testkit::temp_dir_in_workspace;

    #[test]
    fn test_cursor_zero_rejected() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join("rules/test.md"), "line1\nline2\nline3").unwrap();

        // cursor=0 should be rejected
        let input = RulesPageInput {
            path: "rules/test.md".to_string(),
            cursor: Some("0".to_string()),
            max_lines: 10,
        };

        let result = rules_page(input, project_root);
        assert!(result.is_err(), "Should reject cursor=0");
    }

    #[test]
    fn test_empty_file_cursor_1() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join("rules/empty.md"), "").unwrap();

        // cursor=1 on empty file should be allowed (Phase 5 behavior)
        let input = RulesPageInput {
            path: "rules/empty.md".to_string(),
            cursor: Some("1".to_string()),
            max_lines: 10,
        };

        let result = rules_page(input, project_root);
        assert!(result.is_ok(), "Should allow cursor=1 on empty file");

        let output = result.unwrap();
        assert_eq!(
            output.start_line, 1,
            "Empty file should start at line 1 (1-indexed)"
        );
        assert_eq!(
            output.end_line, 0,
            "Empty file should end at line 0 (empty range)"
        );
        assert_eq!(output.total_lines, 0, "Empty file has 0 lines");
    }

    #[test]
    fn test_empty_file_no_cursor() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join("rules/empty.md"), "").unwrap();

        // No cursor on empty file should return start_line=1, end_line=0 (Phase 5 behavior)
        let input = RulesPageInput {
            path: "rules/empty.md".to_string(),
            cursor: None,
            max_lines: 10,
        };

        let result = rules_page(input, project_root);
        assert!(result.is_ok(), "Should allow no cursor on empty file");

        let output = result.unwrap();
        assert_eq!(output.content, "", "Should return empty content");
        assert_eq!(
            output.start_line, 1,
            "Empty file should start at line 1 (1-indexed)"
        );
        assert_eq!(
            output.end_line, 0,
            "Empty file should end at line 0 (empty range)"
        );
        assert_eq!(output.total_lines, 0, "Should have 0 total lines");
        assert!(!output.has_more, "Should not have more lines");
    }
}

#[cfg(test)]
mod bounds_tests {
    use super::*;
    use std::fs;
    use typstlab_testkit::temp_dir_in_workspace;

    #[test]
    fn test_rules_list_offset_exceeds_total() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join("rules/test.md"), "content").unwrap();

        // Offset beyond total files
        let input = RulesListInput {
            scope: RulesScope::Project,
            paper_id: None,
            subdir: None,
            cursor: Some("100".to_string()), // offset > total
            limit: 10,
        };

        let result = rules_list(input, project_root);
        assert!(result.is_err(), "Should reject offset > total");
    }

    #[test]
    fn test_rules_list_offset_equals_total() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join("rules/test.md"), "content").unwrap();

        // Offset equals total (should return empty results, not error)
        let input = RulesListInput {
            scope: RulesScope::Project,
            paper_id: None,
            subdir: None,
            cursor: Some("1".to_string()), // offset == total
            limit: 10,
        };

        let result = rules_list(input, project_root);
        assert!(result.is_ok(), "Should allow offset == total");
        let output = result.unwrap();
        assert_eq!(output.files.len(), 0, "Should return empty results");
    }

    #[test]
    fn test_rules_page_cursor_exceeds_total() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join("rules/test.md"), "line1\nline2\nline3").unwrap();

        // Cursor beyond total lines
        let input = RulesPageInput {
            path: "rules/test.md".to_string(),
            cursor: Some("1000".to_string()), // cursor > total lines
            max_lines: 10,
        };

        let result = rules_page(input, project_root);
        assert!(result.is_err(), "Should reject cursor > total lines");
    }

    #[test]
    fn test_rules_page_cursor_equals_total() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join("rules/test.md"), "line1\nline2\nline3").unwrap();

        // Cursor equals total lines (should return the last line)
        let input = RulesPageInput {
            path: "rules/test.md".to_string(),
            cursor: Some("3".to_string()), // cursor == total lines
            max_lines: 10,
        };

        let result = rules_page(input, project_root);
        assert!(result.is_ok(), "Should allow cursor == total lines");
        let output = result.unwrap();
        assert_eq!(output.content, "line3", "Should return the last line");
        assert_eq!(output.start_line, 3);
        assert_eq!(output.end_line, 3);
        assert!(!output.has_more, "Should indicate no more lines");
    }

    #[test]
    fn test_rules_page_cursor_beyond_total() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join("rules/test.md"), "line1\nline2\nline3").unwrap();

        // Cursor beyond total lines (should error)
        let input = RulesPageInput {
            path: "rules/test.md".to_string(),
            cursor: Some("4".to_string()), // cursor > total lines
            max_lines: 10,
        };

        let result = rules_page(input, project_root);
        assert!(result.is_err(), "Should reject cursor beyond total lines");
    }

    #[test]
    fn test_read_lines_range_correct_total() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        // Create file with exactly 10 lines
        fs::create_dir_all(project_root.join("rules")).unwrap();
        let content = (1..=10)
            .map(|i| format!("line{}", i))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(project_root.join("rules/test.md"), content).unwrap();

        // Read first 5 lines
        let path = project_root.join("rules/test.md");
        let (_, actual_lines, total_lines, has_more) = read_lines_range(&path, 1, 5).unwrap();

        assert_eq!(actual_lines, 5, "Should read 5 lines");
        assert_eq!(total_lines, 10, "Should count all 10 lines");
        assert!(has_more, "Should indicate more lines exist");
    }

    #[test]
    fn test_read_lines_range_read_all() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        // Create file with exactly 5 lines
        fs::create_dir_all(project_root.join("rules")).unwrap();
        let content = (1..=5)
            .map(|i| format!("line{}", i))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(project_root.join("rules/test.md"), content).unwrap();

        // Read all lines (request more than available)
        let path = project_root.join("rules/test.md");
        let (_, actual_lines, total_lines, has_more) = read_lines_range(&path, 1, 10).unwrap();

        assert_eq!(actual_lines, 5, "Should read all 5 lines");
        assert_eq!(total_lines, 5, "Should count all 5 lines");
        assert!(!has_more, "Should indicate no more lines");
    }
}

#[cfg(test)]
mod security_tests_v3 {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use typstlab_testkit::temp_dir_in_workspace;

    #[test]
    fn test_empty_paper_id_blocked() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        // Should block: papers//rules/file.md (empty paper_id)
        let result = validate_rules_path(project_root, Path::new("papers//rules/file.md"));
        assert!(result.is_err(), "Should block empty paper_id");

        // Should block: papers///rules/file.md (multiple slashes)
        let result = validate_rules_path(project_root, Path::new("papers///rules/file.md"));
        assert!(result.is_err(), "Should block multiple slashes");
    }

    #[test]
    fn test_cross_platform_path_handling() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        fs::create_dir_all(project_root.join("papers/paper1/rules")).unwrap();
        fs::write(project_root.join("papers/paper1/rules/guide.md"), "content").unwrap();

        // Use Path API for cross-platform path construction
        let path = PathBuf::from("papers")
            .join("paper1")
            .join("rules")
            .join("guide.md");
        let result = validate_rules_path(project_root, &path);
        assert!(
            result.is_ok(),
            "Should accept path constructed with Path API"
        );
    }

    #[test]
    fn test_curdir_prefix_rules() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join("rules/test.md"), "content").unwrap();

        // Should accept ./rules/test.md
        let result = validate_rules_path(project_root, Path::new("./rules/test.md"));
        assert!(result.is_ok(), "Should accept ./rules/ prefix");

        // Verify canonicalized path is correct
        let canonical = result.unwrap();
        assert!(canonical.ends_with("rules/test.md"));
    }

    #[test]
    fn test_curdir_prefix_papers() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        fs::create_dir_all(project_root.join("papers/paper1/rules")).unwrap();
        fs::write(project_root.join("papers/paper1/rules/guide.md"), "content").unwrap();

        // Should accept ./papers/paper1/rules/guide.md
        let result = validate_rules_path(project_root, Path::new("./papers/paper1/rules/guide.md"));
        assert!(result.is_ok(), "Should accept ./papers/ prefix");

        // Verify canonicalized path is correct
        let canonical = result.unwrap();
        assert!(canonical.ends_with("papers/paper1/rules/guide.md"));
    }

    #[test]
    fn test_multiple_curdir_normalized() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join("rules/test.md"), "content").unwrap();

        // Should handle ././rules/test.md (multiple .)
        let result = validate_rules_path(project_root, Path::new("././rules/test.md"));
        assert!(result.is_ok(), "Should handle multiple . components");
    }

    #[test]
    fn test_curdir_does_not_bypass_security() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        // ./../../etc/passwd should still be blocked
        let result = validate_rules_path(project_root, Path::new("./../../etc/passwd"));
        assert!(
            result.is_err(),
            "Should still block parent traversal with ./"
        );

        // ./../.env should still be blocked
        let result = validate_rules_path(project_root, Path::new("./../.env"));
        assert!(result.is_err(), "Should still block parent traversal");

        // ./.env should be blocked (not in rules/ or papers/)
        let result = validate_rules_path(project_root, Path::new("./.env"));
        assert!(
            result.is_err(),
            "Should block ./.env (not in allowed directories)"
        );
    }
}

#[cfg(test)]
mod correctness_tests_v3 {
    use super::*;
    use std::fs;
    use typstlab_testkit::temp_dir_in_workspace;

    #[test]
    fn test_empty_file_with_cursor_1_allowed() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join("rules/empty.md"), "").unwrap();

        // Empty file should allow cursor=1 for client stability
        let input = RulesPageInput {
            path: "rules/empty.md".to_string(),
            cursor: Some("1".to_string()),
            max_lines: 10,
        };

        let result = rules_page(input, project_root);
        assert!(result.is_ok(), "Should allow cursor=1 on empty file");

        let output = result.unwrap();
        assert_eq!(
            output.start_line, 1,
            "Empty file should start at line 1 (1-indexed)"
        );
        assert_eq!(
            output.end_line, 0,
            "Empty file should end at line 0 (empty range)"
        );
        assert_eq!(output.total_lines, 0, "Empty file has 0 lines");
        assert_eq!(output.content, "", "Content should be empty");
        assert!(!output.has_more, "Should not have more");
        assert!(output.next_cursor.is_none(), "No next cursor");
    }

    #[test]
    fn test_empty_file_no_cursor_returns_consistent_range() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join("rules/empty.md"), "").unwrap();

        // Empty file with no cursor should return start_line=1, end_line=0 (1-indexed with empty range)
        let input = RulesPageInput {
            path: "rules/empty.md".to_string(),
            cursor: None,
            max_lines: 10,
        };

        let result = rules_page(input, project_root);
        assert!(result.is_ok(), "Should allow no cursor on empty file");

        let output = result.unwrap();
        assert_eq!(
            output.start_line, 1,
            "Empty file should start at line 1 (1-indexed)"
        );
        assert_eq!(
            output.end_line, 0,
            "Empty file should end at line 0 (empty range)"
        );
        assert_eq!(output.total_lines, 0, "Should have 0 total lines");
        assert_eq!(output.content, "", "Content should be empty");
        assert!(!output.has_more, "Should not have more lines");
        assert!(output.next_cursor.is_none(), "Should not have next cursor");
    }

    #[test]
    fn test_empty_file_arbitrary_cursor_rejected() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join("rules/empty.md"), "").unwrap();

        // Empty file should reject arbitrary cursors like cursor=5
        let input = RulesPageInput {
            path: "rules/empty.md".to_string(),
            cursor: Some("5".to_string()),
            max_lines: 10,
        };

        let result = rules_page(input, project_root);
        assert!(
            result.is_err(),
            "Should reject arbitrary cursor on empty file"
        );

        // Verify error message mentions cursor=1 requirement
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Empty file") && err_msg.contains("cursor must be 1"),
            "Error message should mention cursor=1 requirement, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_max_lines_zero_rejected() {
        let temp = temp_dir_in_workspace();
        let project_root = temp.path();

        fs::create_dir_all(project_root.join("rules")).unwrap();
        fs::write(project_root.join("rules/test.md"), "line1\nline2\nline3").unwrap();

        // max_lines=0 should be rejected
        let input = RulesPageInput {
            path: "rules/test.md".to_string(),
            cursor: None,
            max_lines: 0,
        };

        let result = rules_page(input, project_root);
        assert!(result.is_err(), "Should reject max_lines=0");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("must be at least 1"),
            "Error message should mention minimum requirement, got: {}",
            err_msg
        );
    }
}
