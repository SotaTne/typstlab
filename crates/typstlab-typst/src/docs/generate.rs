//! Markdown file generation from docs.json
//!
//! Converts parsed docs.json structure to hierarchical Markdown files.

use super::html_to_md;
use super::render_bodies;
use super::render_func;
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

    // YAML Frontmatter
    if let Some(desc) = &entry.description {
        markdown.push_str("---\n");
        markdown.push_str("description: |\n");
        // Indent each line with 2 spaces for YAML block scalar
        for line in desc.lines() {
            markdown.push_str("  ");
            markdown.push_str(line);
            markdown.push('\n');
        }
        markdown.push_str("---\n\n");
    }

    // Title (AFTER frontmatter)
    markdown.push_str(&format!("# {}\n\n", entry.title));

    // Body content (with body kind routing)
    if let Some(body) = &entry.body {
        match body.kind.as_str() {
            "html" => {
                // HTML content: convert to Markdown
                let html = body.as_html()?;
                let converted = html_to_md::convert(html)?;
                // Remove duplicate h1 if it matches title
                let cleaned = remove_duplicate_heading(&converted, &entry.title);
                markdown.push_str(&cleaned);
                markdown.push_str("\n\n");
            }
            "func" => {
                // Function definition: render with specialized function
                let func_md = render_func::render_func_body(&body.content)?;
                markdown.push_str(&func_md);
                markdown.push_str("\n\n");
            }
            "type" => {
                // Type definition: render with specialized function
                let type_md = render_bodies::render_type_body(&body.content)?;
                markdown.push_str(&type_md);
                markdown.push_str("\n\n");
            }
            "category" => {
                // Category listing: render with specialized function
                let cat_md = render_bodies::render_category_body(&body.content)?;
                markdown.push_str(&cat_md);
                markdown.push_str("\n\n");
            }
            "group" => {
                // Function group: render with specialized function
                let group_md = render_bodies::render_group_body(&body.content)?;
                markdown.push_str(&group_md);
                markdown.push_str("\n\n");
            }
            "symbols" => {
                // Symbol table: render with specialized function
                let sym_md = render_bodies::render_symbols_body(&body.content)?;
                markdown.push_str(&sym_md);
                markdown.push_str("\n\n");
            }
            _ => {
                // Unknown kind: render warning comment
                markdown.push_str(&format!("<!-- Unknown body kind: {} -->\n\n", body.kind));
            }
        }
    }

    Ok(markdown)
}

/// Remove duplicate h1 heading if it matches the title
///
/// Checks if the first non-empty line is an h1 heading that matches the title.
/// If so, removes it and any following empty lines.
fn remove_duplicate_heading(content: &str, title: &str) -> String {
    let title_normalized = title.trim().to_lowercase();
    let lines: Vec<&str> = content.lines().collect();

    // Check if first non-empty line is h1 matching title
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Check for h1 (starts with # but not ##)
        if trimmed.starts_with("# ") && !trimmed.starts_with("## ") {
            let heading_text = trimmed[2..].trim().to_lowercase();
            if heading_text == title_normalized {
                // Skip this line and following empty lines
                let mut skip_until = i + 1;
                while skip_until < lines.len() && lines[skip_until].trim().is_empty() {
                    skip_until += 1;
                }
                return lines[skip_until..].join("\n");
            }
        }

        // Non-empty, non-matching line found - no duplicate
        break;
    }

    content.to_string()
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

    /// Test: YAML frontmatter generation from fixture
    #[test]
    fn test_yaml_frontmatter_from_fixture() {
        // Load fixture JSON
        let fixture_json = include_str!("../../../../fixtures/typst/v0.12.0/overview.json");
        let entry: DocsEntry =
            serde_json::from_str(fixture_json).expect("Failed to parse overview.json");

        // Generate Markdown
        let result = entry_to_markdown(&entry).expect("Failed to generate markdown");

        // Load expected output
        let expected = include_str!("../../../../fixtures/typst/v0.12.0/overview.md");

        // Compare (trim whitespace for comparison)
        assert_eq!(
            result.trim(),
            expected.trim(),
            "Generated markdown should match expected output"
        );

        // Verify YAML frontmatter present
        assert!(
            result.starts_with("---\n"),
            "Should start with YAML frontmatter"
        );
        assert!(
            result.contains("description: |"),
            "Should have description field"
        );

        // Verify no duplicate h1
        let h1_count = result.matches("\n# ").count();
        assert_eq!(h1_count, 1, "Should have exactly one h1 heading");
    }

    /// Test: Duplicate heading removal
    #[test]
    fn test_duplicate_heading_removal() {
        let content = "# Overview\n\nSome content...";
        let title = "Overview";

        let result = remove_duplicate_heading(content, title);

        assert!(
            !result.starts_with("# Overview"),
            "Should remove duplicate h1"
        );
        assert!(
            result.starts_with("Some content..."),
            "Should start with actual content"
        );
    }

    /// Test: Duplicate heading removal (case insensitive)
    #[test]
    fn test_duplicate_heading_removal_case_insensitive() {
        let content = "# overview\n\nSome content...";
        let title = "Overview";

        let result = remove_duplicate_heading(content, title);

        assert!(
            !result.starts_with("# overview"),
            "Should remove duplicate h1 (case insensitive)"
        );
        assert!(
            result.starts_with("Some content..."),
            "Should start with actual content"
        );
    }

    /// Test: No duplicate heading removal if different
    #[test]
    fn test_no_duplicate_heading_removal_if_different() {
        let content = "# Introduction\n\nSome content...";
        let title = "Overview";

        let result = remove_duplicate_heading(content, title);

        assert_eq!(result, content, "Should not remove non-duplicate heading");
    }

    /// Test: Integration - Function entry to markdown
    #[test]
    fn test_integration_func_to_markdown() {
        let fixture =
            include_str!("../../../../fixtures/typst/v0.12.0/test-fixtures/func-assert.json");
        let entry: DocsEntry =
            serde_json::from_str(fixture).expect("Failed to parse func-assert.json");

        let result = entry_to_markdown(&entry).expect("Failed to generate markdown");

        // Verify title (no YAML frontmatter for this entry as it has no description)
        assert!(result.contains("# Assert"), "Should have title");

        // Verify function body rendered
        assert!(
            result.contains("## Signature"),
            "Should have Signature section"
        );
        assert!(
            result.contains("## Parameters"),
            "Should have Parameters section"
        );
        assert!(
            result.contains("**condition**"),
            "Should have condition parameter"
        );
    }

    /// Test: Integration - Type entry to markdown
    #[test]
    fn test_integration_type_to_markdown() {
        let fixture =
            include_str!("../../../../fixtures/typst/v0.12.0/test-fixtures/type-arguments.json");
        let entry: DocsEntry =
            serde_json::from_str(fixture).expect("Failed to parse type-arguments.json");

        let result = entry_to_markdown(&entry).expect("Failed to generate markdown");

        // Verify title
        assert!(result.contains("# Arguments"), "Should have title");

        // Verify type body rendered
        assert!(
            result.contains("## Constructor"),
            "Should have Constructor section"
        );
        assert!(result.contains("## Methods"), "Should have Methods section");
    }

    /// Test: Integration - Category entry to markdown
    #[test]
    fn test_integration_category_to_markdown() {
        let fixture = include_str!(
            "../../../../fixtures/typst/v0.12.0/test-fixtures/category-foundations.json"
        );
        let entry: DocsEntry =
            serde_json::from_str(fixture).expect("Failed to parse category-foundations.json");

        let result = entry_to_markdown(&entry).expect("Failed to generate markdown");

        // Verify title
        assert!(result.contains("# Foundations"), "Should have title");

        // Verify category body rendered
        assert!(result.contains("## Items"), "Should have Items section");
        assert!(result.contains("- ["), "Should have list items");
    }
}
