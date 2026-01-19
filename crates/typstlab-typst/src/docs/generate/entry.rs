//! DocsEntry body content to Markdown conversion

use crate::docs::{html_to_md, render_bodies, render_func, schema::DocsEntry};
use thiserror::Error;

/// Generates Markdown content from DocsEntry body
///
/// Routes body content based on `kind` field:
/// - `html`: Convert HTML to Markdown (with duplicate h1 removal)
/// - `func`: Render function definition
/// - `type`: Render type definition
/// - `category`: Render category listing
/// - `group`: Render function group
/// - `symbols`: Render symbol table
///
/// # Arguments
///
/// * `entry` - DocsEntry to convert
///
/// # Returns
///
/// Markdown content string (without YAML frontmatter)
///
/// # Errors
///
/// Returns error if:
/// - HTML conversion fails
/// - Body rendering fails
/// - JSON parsing fails
///
/// # Examples
///
/// ```no_run
/// use typstlab_typst::docs::{generate::generate_body_markdown, schema::DocsEntry};
///
/// let entry: DocsEntry = serde_json::from_str(r#"{"route": "/DOCS-BASE/", ...}"#).unwrap();
/// let markdown = generate_body_markdown(&entry).unwrap();
/// ```
pub fn generate_body_markdown(entry: &DocsEntry) -> Result<String, BodyRenderError> {
    let mut markdown = String::new();

    // Calculate depth relative to root for link rewriting
    let relative_route = entry
        .route
        .strip_prefix("/DOCS-BASE/")
        .unwrap_or(&entry.route);

    let depth = relative_route.trim_end_matches('/').matches('/').count();

    if let Some(body) = &entry.body {
        match body.kind.as_str() {
            "html" => {
                // HTML content: convert to Markdown
                let html = body.as_html()?;
                let converted = html_to_md::convert(html, depth)?;
                // Remove duplicate h1 if it matches title
                let cleaned = remove_duplicate_heading(&converted, &entry.title);
                markdown.push_str(&cleaned);
                markdown.push_str("\n\n");
            }
            "func" => {
                // Function definition: render with specialized function
                let func_md = render_func::render_func_body(&body.content, depth)?;
                markdown.push_str(&func_md);
                markdown.push_str("\n\n");
            }
            "type" => {
                // Type definition: render with specialized function
                let type_md = render_bodies::render_type_body(&body.content, depth)?;
                markdown.push_str(&type_md);
                markdown.push_str("\n\n");
            }
            "category" => {
                // Category listing: render with specialized function
                let cat_md = render_bodies::render_category_body(&body.content, depth)?;
                markdown.push_str(&cat_md);
                markdown.push_str("\n\n");
            }
            "group" => {
                // Function group: render with specialized function
                let group_md = render_bodies::render_group_body(&body.content, depth)?;
                markdown.push_str(&group_md);
                markdown.push_str("\n\n");
            }
            "symbols" => {
                // Symbol table: render with specialized function
                let sym_md = render_bodies::render_symbols_body(&body.content, depth)?;
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
///
/// # Arguments
///
/// * `content` - Markdown content with possible duplicate h1
/// * `title` - Expected title to check against
///
/// # Returns
///
/// Cleaned markdown content
///
/// # Examples
///
/// ```
/// use typstlab_typst::docs::generate::remove_duplicate_heading;
///
/// let content = "# Overview\n\nSome content...";
/// let result = remove_duplicate_heading(content, "Overview");
/// assert!(!result.starts_with("# Overview"));
/// ```
pub fn remove_duplicate_heading(content: &str, title: &str) -> String {
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

/// Body rendering errors
#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum BodyRenderError {
    /// Schema error
    #[error("Schema error: {0}")]
    SchemaError(#[from] crate::docs::schema::SchemaError),

    /// HTML conversion error
    #[error("HTML conversion error: {0}")]
    HtmlConversionError(#[from] html_to_md::ConversionError),

    /// JSON parsing error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Function rendering error
    #[error("Function rendering error: {0}")]
    FuncRenderError(#[from] render_func::RenderError),

    /// Body type rendering error
    #[error("Body type rendering error: {0}")]
    BodyTypeRenderError(#[from] render_bodies::RenderError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Helper: Create test DocsEntry
    fn test_entry(title: &str, html_body: Option<&str>) -> DocsEntry {
        DocsEntry {
            route: "/DOCS-BASE/test".to_string(),
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

    #[test]
    fn test_no_duplicate_heading_removal_if_different() {
        let content = "# Introduction\n\nSome content...";
        let title = "Overview";

        let result = remove_duplicate_heading(content, title);

        assert_eq!(result, content, "Should not remove non-duplicate heading");
    }

    #[test]
    fn test_generate_body_markdown_html() {
        let entry = test_entry("Test", Some("<p>Hello World</p>"));

        let result = generate_body_markdown(&entry).expect("Should generate markdown");

        assert!(result.contains("Hello World"));
    }

    #[test]
    fn test_generate_body_markdown_no_body() {
        let entry = test_entry("Test", None);

        let result = generate_body_markdown(&entry).expect("Should generate empty markdown");

        assert_eq!(result, "");
    }

    #[test]
    fn test_integration_func_body() {
        let fixture =
            include_str!("../../../../../fixtures/typst/v0.12.0/test-fixtures/func-assert.json");
        let entry: DocsEntry =
            serde_json::from_str(fixture).expect("Failed to parse func-assert.json");

        let result = generate_body_markdown(&entry).expect("Failed to generate markdown");

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

    #[test]
    fn test_integration_type_body() {
        let fixture =
            include_str!("../../../../../fixtures/typst/v0.12.0/test-fixtures/type-arguments.json");
        let entry: DocsEntry =
            serde_json::from_str(fixture).expect("Failed to parse type-arguments.json");

        let result = generate_body_markdown(&entry).expect("Failed to generate markdown");

        // Verify type body rendered
        assert!(
            result.contains("## Constructor"),
            "Should have Constructor section"
        );
        assert!(result.contains("## Methods"), "Should have Methods section");
    }

    #[test]
    fn test_integration_category_body() {
        let fixture = include_str!(
            "../../../../../fixtures/typst/v0.12.0/test-fixtures/category-foundations.json"
        );
        let entry: DocsEntry =
            serde_json::from_str(fixture).expect("Failed to parse category-foundations.json");

        let result = generate_body_markdown(&entry).expect("Failed to generate markdown");

        // Verify category body rendered
        assert!(result.contains("## Items"), "Should have Items section");
        assert!(result.contains("- ["), "Should have list items");
    }

    #[test]
    fn test_integration_nested_link() {
        // Creates an entry that is deeply nested: /DOCS-BASE/reference/math/attach
        // This simulates depth = 3.
        let mut entry = test_entry(
            "Nested",
            Some(r#"<a href="/DOCS-BASE/reference/math/stretch">Stretch</a>"#),
        );
        entry.route = "/DOCS-BASE/reference/math/attach".to_string();

        let result = generate_body_markdown(&entry).expect("Failed to generate markdown");

        // Depth 2 (reference/math/attach) should produce "../../" prefix
        assert!(
            result.contains("../../reference/math/stretch.md"),
            "Should have correct relative link for nested document, got: {}",
            result
        );
    }
}
