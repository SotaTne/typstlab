//! Markdown file generation from docs.json
//!
//! Converts parsed docs.json structure to hierarchical Markdown files.

mod entry;
mod frontmatter;
mod route;

// Re-export public APIs
pub use entry::{generate_body_markdown, remove_duplicate_heading};
pub use frontmatter::generate_frontmatter;
pub use route::route_to_filepath;

use crate::docs::schema::DocsEntry;
use std::fs;
use std::path::Path;
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
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use typstlab_typst::docs::{generate::generate_markdown_files, schema::DocsEntry};
///
/// let entries: Vec<DocsEntry> = vec![]; // Load from JSON
/// let count = generate_markdown_files(&entries, Path::new(".typstlab/kb/typst/docs"), false).unwrap();
/// println!("Generated {} files", count);
/// ```
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

/// Converts DocsEntry to complete Markdown file content
///
/// Combines YAML frontmatter and body content.
fn entry_to_markdown(entry: &DocsEntry) -> Result<String, GenerateError> {
    // Generate YAML frontmatter
    let frontmatter = generate_frontmatter(&entry.title, entry.description.as_deref())?;

    // Generate body content
    let body_markdown = generate_body_markdown(entry)?;

    // Combine frontmatter and body
    Ok(format!("{}{}", frontmatter, body_markdown))
}

/// Markdown generation errors
#[derive(Debug, Error)]
pub enum GenerateError {
    /// Route conversion error
    #[error("Route error: {0}")]
    RouteError(#[from] route::RouteError),

    /// Frontmatter generation error
    #[error("Frontmatter error: {0}")]
    FrontmatterError(#[from] frontmatter::FrontmatterError),

    /// Body rendering error
    #[error("Body rendering error: {0}")]
    BodyRenderError(#[from] entry::BodyRenderError),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
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
            body: html_body.map(|html| crate::docs::schema::Body {
                kind: "html".to_string(),
                content: serde_json::Value::String(html.to_string()),
            }),
            children: vec![],
            extra: HashMap::new(),
        }
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
        // Title is in YAML frontmatter, not as h1 in body
        assert!(content.starts_with("---\n"), "Should have YAML frontmatter");
        assert!(
            content.contains("title: Overview"),
            "Should have title in frontmatter"
        );
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

    /// Test: YAML frontmatter generation from fixture
    #[test]
    fn test_yaml_frontmatter_from_fixture() {
        // Load fixture JSON
        let fixture_json = include_str!("../../../../../fixtures/typst/v0.12.0/overview.json");
        let entry: DocsEntry =
            serde_json::from_str(fixture_json).expect("Failed to parse overview.json");

        // Generate Markdown
        let result = entry_to_markdown(&entry).expect("Failed to generate markdown");

        // Load expected output
        let expected = include_str!("../../../../../fixtures/typst/v0.12.0/overview.md");

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
            result.contains("title: Overview"),
            "Should have title field in frontmatter"
        );
        assert!(
            result.contains("description: |"),
            "Should have description field"
        );

        // Verify no h1 in body (title is in YAML frontmatter)
        let h1_count = result.matches("\n# ").count();
        assert_eq!(
            h1_count, 0,
            "Should have no h1 headings in body (title in frontmatter)"
        );
    }

    /// Test: Integration - Full entry to markdown
    #[test]
    fn test_integration_entry_to_markdown() {
        let fixture =
            include_str!("../../../../../fixtures/typst/v0.12.0/test-fixtures/func-assert.json");
        let entry: DocsEntry =
            serde_json::from_str(fixture).expect("Failed to parse func-assert.json");

        let result = entry_to_markdown(&entry).expect("Failed to generate markdown");

        // Verify title in YAML frontmatter (always present)
        assert!(result.starts_with("---\n"), "Should have YAML frontmatter");
        assert!(
            result.contains("title: Assert"),
            "Should have title in frontmatter"
        );

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
}
