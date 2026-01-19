use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use typstlab_core::path::has_absolute_or_rooted_component;

#[derive(Debug, Deserialize)]
pub struct DocsBrowseInput {
    pub path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DocsBrowseOutput {
    pub items: Vec<DocsEntry>,
}

#[derive(Debug, Serialize)]
pub struct DocsEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub entry_type: String, // "file" or "directory"
    pub path: String, // Relative path from docs root
}

/// Browse documentation directory structure
pub fn docs_browse(input: DocsBrowseInput, project_root: &Path) -> Result<DocsBrowseOutput> {
    let docs_root = project_root.join(".typstlab/kb/typst/docs");

    // Determine target directory
    let target_rel_path = if let Some(p) = input.path {
        if p.is_empty() {
            PathBuf::new()
        } else {
            PathBuf::from(p)
        }
    } else {
        PathBuf::new()
    };

    // Security check
    if has_absolute_or_rooted_component(&target_rel_path)
        || target_rel_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        bail!("Invalid path: cannot traverse outside docs root");
    }

    let target_abs_path = docs_root.join(&target_rel_path);

    if !target_abs_path.exists() {
        bail!("Path not found: {}", target_rel_path.display());
    }

    if !target_abs_path.is_dir() {
        bail!("Path is not a directory: {}", target_rel_path.display());
    }

    let mut items = Vec::new();

    for entry in std::fs::read_dir(target_abs_path)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files
        if file_name.starts_with('.') {
            continue;
        }

        let entry_type = if path.is_dir() { "directory" } else { "file" };

        let relative_path = target_rel_path
            .join(&file_name)
            .to_string_lossy()
            .to_string();

        items.push(DocsEntry {
            name: file_name,
            entry_type: entry_type.to_string(),
            path: relative_path,
        });
    }

    // Sort items for deterministic output (directories first, then files)
    items.sort_by(|a, b| {
        if a.entry_type == b.entry_type {
            a.name.cmp(&b.name)
        } else if a.entry_type == "directory" {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    Ok(DocsBrowseOutput { items })
}

#[derive(Debug, Deserialize)]
pub struct DocsSearchInput {
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct DocsSearchOutput {
    pub matches: Vec<SearchMatch>,
}

#[derive(Debug, Serialize)]
pub struct SearchMatch {
    pub path: String,
    pub line: usize,
    pub excerpt: String,
}

/// Search within documentation files
pub fn docs_search(input: DocsSearchInput, project_root: &Path) -> Result<DocsSearchOutput> {
    let docs_root = project_root.join(".typstlab/kb/typst/docs");

    if !docs_root.exists() {
        return Ok(DocsSearchOutput { matches: vec![] });
    }

    let mut matches = Vec::new();
    let query = input.query.to_lowercase();

    // Determine implementation strategy:
    // For v0.1, we'll use a simple recursive walk and string contains check.
    // This is not efficient for large codebases but fine for typical docs.

    for entry in walkdir::WalkDir::new(&docs_root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        // Check extension
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        // Read content
        // Ignore read errors (e.g. non-utf8)
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (line_idx, line) in content.lines().enumerate() {
            if line.to_lowercase().contains(&query) {
                let relative_path = path
                    .strip_prefix(&docs_root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();

                matches.push(SearchMatch {
                    path: relative_path,
                    line: line_idx + 1,
                    excerpt: line.trim().chars().take(100).collect(), // Truncate excerpt
                });

                if matches.len() >= 50 {
                    // Safety limit
                    break;
                }
            }
        }

        if matches.len() >= 50 {
            break;
        }
    }

    Ok(DocsSearchOutput { matches })
}
