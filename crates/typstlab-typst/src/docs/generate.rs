//! Markdown file generation from docs.json
//!
//! Converts parsed docs.json structure to hierarchical Markdown files.

use super::html_to_md;
use super::schema::{DocsEntry, SchemaError};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Generates Markdown files from docs.json entries
///
/// # Arguments
///
/// * `entries` - Parsed docs.json entries
/// * `target_dir` - Directory to write Markdown files to
/// * `verbose` - Enable verbose output
///
/// # Returns
///
/// Number of files generated
///
/// # Errors
///
/// Returns error if:
/// - Path validation fails (traversal, absolute paths)
/// - File I/O fails
/// - HTML conversion fails
pub fn generate_markdown_files(
    entries: &[DocsEntry],
    target_dir: &Path,
    verbose: bool,
) -> Result<usize, GenerateError> {
    let mut file_count = 0;

    for entry in entries {
        file_count += generate_entry(entry, target_dir, verbose)?;
    }

    Ok(file_count)
}

/// Generates files for a single entry and its children (recursive)
fn generate_entry(
    entry: &DocsEntry,
    target_dir: &Path,
    verbose: bool,
) -> Result<usize, GenerateError> {
    // Convert route to file path (includes validation)
    let file_path = route_to_filepath(target_dir, &entry.route)?;

    // Ensure parent directory exists
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Generate Markdown content
    let markdown = entry_to_markdown(entry)?;

    // Write file
    fs::write(&file_path, markdown)?;

    if verbose {
        eprintln!("Generated: {}", file_path.display());
    }

    let mut count = 1;

    // Recursively process children
    for child in &entry.children {
        count += generate_entry(child, target_dir, verbose)?;
    }

    Ok(count)
}

/// Converts DocsEntry route to file path
///
/// # Security
///
/// - Validates route after removing /DOCS-BASE/ prefix
/// - Blocks absolute paths
/// - Blocks parent directory traversal (..)
/// - Only allows relative paths under target_dir
///
/// # Mapping Rules
///
/// - "/DOCS-BASE/" → "index.md"
/// - "/DOCS-BASE/tutorial/" → "tutorial/index.md"
/// - "/DOCS-BASE/tutorial/writing/" → "tutorial/writing.md"
fn route_to_filepath(target_dir: &Path, route: &str) -> Result<PathBuf, GenerateError> {
    // Remove /DOCS-BASE/ prefix
    let relative_route = route
        .strip_prefix("/DOCS-BASE/")
        .ok_or_else(|| GenerateError::InvalidRoute(route.to_string()))?;

    // Validate the relative route for path traversal
    if !relative_route.is_empty() {
        let route_path = Path::new(relative_route);

        // Check for absolute or rooted paths (after prefix removal)
        if typstlab_core::path::has_absolute_or_rooted_component(route_path) {
            return Err(GenerateError::InvalidRoute(format!(
                "Absolute or rooted path not allowed: {}",
                relative_route
            )));
        }

        // Check for parent directory traversal (..)
        use std::path::Component;
        if route_path
            .components()
            .any(|c| matches!(c, Component::ParentDir))
        {
            return Err(GenerateError::InvalidRoute(format!(
                "Path traversal (..) not allowed: {}",
                relative_route
            )));
        }
    }

    // Convert to path
    let mut path = target_dir.to_path_buf();

    if relative_route.is_empty() {
        // Root: index.md
        path.push("index.md");
    } else if relative_route.ends_with('/') {
        // Directory: dir/index.md
        let dir_name = relative_route.trim_end_matches('/');
        path.push(dir_name);
        path.push("index.md");
    } else {
        // File: dir/file.md
        path.push(format!("{}.md", relative_route));
    }

    Ok(path)
}

/// Converts DocsEntry to Markdown content
fn entry_to_markdown(entry: &DocsEntry) -> Result<String, GenerateError> {
    let mut markdown = String::new();

    // Title
    markdown.push_str(&format!("# {}\n\n", entry.title));

    // Description
    if let Some(desc) = &entry.description {
        markdown.push_str(desc);
        markdown.push_str("\n\n");
    }

    // Body content
    if let Some(body) = &entry.body {
        if body.is_html() {
            // HTML content: convert to Markdown
            let html = body.as_html()?;
            let converted = html_to_md::convert(html)?;
            markdown.push_str(&converted);
            markdown.push_str("\n\n");
        } else if body.is_definition() {
            // Function/type definition: placeholder for now
            // TODO: Implement full function definition formatting
            markdown.push_str("## Definition\n\n");
            markdown.push_str("*Function definition rendering not yet implemented*\n\n");
            markdown.push_str(&format!(
                "```json\n{}\n```\n\n",
                serde_json::to_string_pretty(&body.content)?
            ));
        }
    }

    Ok(markdown)
}

/// Markdown generation errors
#[derive(Debug, Error)]
pub enum GenerateError {
    /// Schema validation error
    #[error("Schema error: {0}")]
    SchemaError(#[from] SchemaError),

    /// Invalid route format
    #[error("Invalid route: {0}")]
    InvalidRoute(String),

    /// HTML conversion error
    #[error("HTML conversion error: {0}")]
    HtmlConversionError(#[from] super::html_to_md::ConversionError),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    /// Helper: Create test DocsEntry
    fn test_entry(route: &str, title: &str, html_body: Option<&str>) -> DocsEntry {
        DocsEntry {
            route: route.to_string(),
            title: title.to_string(),
            description: None,
            part: None,
            outline: vec![],
            body: html_body.map(|html| super::super::schema::Body {
                kind: "html".to_string(),
                content: serde_json::Value::String(html.to_string()),
            }),
            children: vec![],
            extra: HashMap::new(),
        }
    }

    /// Test: Route to filepath mapping
    #[test]
    fn test_route_to_filepath_root() {
        let target = Path::new("/tmp/docs");
        let path = route_to_filepath(target, "/DOCS-BASE/").expect("Should map root");
        assert_eq!(path, Path::new("/tmp/docs/index.md"));
    }

    #[test]
    fn test_route_to_filepath_directory() {
        let target = Path::new("/tmp/docs");
        let path = route_to_filepath(target, "/DOCS-BASE/tutorial/").expect("Should map directory");
        assert_eq!(path, Path::new("/tmp/docs/tutorial/index.md"));
    }

    #[test]
    fn test_route_to_filepath_file() {
        let target = Path::new("/tmp/docs");
        let path =
            route_to_filepath(target, "/DOCS-BASE/tutorial/writing").expect("Should map file");
        assert_eq!(path, Path::new("/tmp/docs/tutorial/writing.md"));
    }

    /// Test: Generate single file
    #[test]
    fn test_generate_single_file() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let entry = test_entry("/DOCS-BASE/", "Overview", Some("<p>Welcome</p>"));

        let count =
            generate_markdown_files(&[entry], temp.path(), false).expect("Should generate file");

        assert_eq!(count, 1);

        let index_path = temp.path().join("index.md");
        assert!(index_path.exists());

        let content = fs::read_to_string(index_path).expect("Should read file");
        assert!(content.contains("# Overview"));
        assert!(content.contains("Welcome"));
    }

    /// Test: Generate nested structure
    #[test]
    fn test_generate_nested_structure() {
        let temp = TempDir::new().expect("Failed to create temp dir");

        let mut parent = test_entry("/DOCS-BASE/tutorial/", "Tutorial", Some("<p>Learn</p>"));
        parent.children = vec![test_entry(
            "/DOCS-BASE/tutorial/writing/",
            "Writing",
            Some("<p>Write</p>"),
        )];

        let count =
            generate_markdown_files(&[parent], temp.path(), false).expect("Should generate files");

        assert_eq!(count, 2);

        // Parent file
        let parent_path = temp.path().join("tutorial").join("index.md");
        assert!(parent_path.exists());

        // Child file
        let child_path = temp
            .path()
            .join("tutorial")
            .join("writing")
            .join("index.md");
        assert!(child_path.exists());
    }

    /// Test: Path traversal blocked
    #[test]
    fn test_path_traversal_blocked() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let entry = test_entry("../../../etc/passwd", "Malicious", None);

        let result = generate_markdown_files(&[entry], temp.path(), false);
        assert!(result.is_err());
    }

    /// Test: Absolute path blocked
    #[test]
    fn test_absolute_path_blocked() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let entry = test_entry("/tmp/malicious", "Malicious", None);

        let result = generate_markdown_files(&[entry], temp.path(), false);
        assert!(result.is_err());
    }
}
