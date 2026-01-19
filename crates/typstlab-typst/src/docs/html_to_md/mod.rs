//! HTML to Markdown converter for Typst documentation
//!
//! Converts HTML content from docs.json to clean Markdown suitable for LLMs.
//! Uses 2-stage pipeline: HTML → mdast → Markdown for CommonMark compliance.

pub mod render;

use super::html_to_mdast;
use markdown::mdast::Node;
use thiserror::Error;

/// Maximum HTML size per page (5MB)
const MAX_HTML_SIZE: usize = 5_000_000;

/// Converts HTML to Markdown (2-stage pipeline)
///
/// # Architecture
///
/// - Stage 1: HTML → mdast (via html_to_mdast::convert)
/// - Stage 2: mdast → Markdown (via CompositeRenderer)
///
/// CompositeRenderer provides unified rendering:
/// - Table nodes → StructuralTableRenderer (structural GFM)
/// - Other nodes → StandardRenderer (mdast_util_to_markdown)
/// - Assembly → Compositor (structure → final Markdown)
///
/// # Fallback Strategy
///
/// If rendering fails, falls back to plain text extraction from mdast AST.
/// This ensures graceful degradation.
///
/// # Arguments
///
/// * `html` - HTML string to convert
///
/// # Errors
///
/// Returns error if:
/// - HTML exceeds size limit (5MB)
/// - HTML parsing fails
/// - mdast construction fails
///
/// Note: Rendering errors trigger fallback (not error return)
///
/// # Example
///
/// ```
/// use typstlab_typst::docs::html_to_md::convert;
///
/// let html = "<p>Hello</p>";
/// let md = convert(html, 1).expect("Should convert");
/// assert_eq!(md, "Hello");
/// ```
pub fn convert(html: &str, depth: usize) -> Result<String, ConversionError> {
    // Validate HTML size before parsing
    if html.len() > MAX_HTML_SIZE {
        return Err(ConversionError::HtmlTooLarge(html.len()));
    }

    // Stage 1: HTML → mdast
    let mdast = html_to_mdast::convert(html, depth)?;

    // Stage 2: mdast → Markdown (via CompositeRenderer)
    let renderer = render::create_composite_renderer();
    match renderer.render(&mdast) {
        Ok(md) => Ok(md),
        Err(_e) => {
            // Rendering failed: fallback to plain text
            // eprintln!(
            //     "CompositeRenderer failed: {}, falling back to plain text",
            //     e
            // );
            Ok(extract_plain_text(&mdast))
        }
    }
}

/// Extracts plain text from mdast AST (fallback for mdast_util_to_markdown failures)
///
/// Recursively walks mdast nodes and collects all text content.
/// This provides safe degradation when mdast_util_to_markdown fails.
///
/// # Arguments
///
/// * `node` - mdast Node to extract text from
///
/// # Returns
///
/// Plain text string with basic formatting preserved
fn extract_plain_text(node: &Node) -> String {
    match node {
        Node::Root(root) => root
            .children
            .iter()
            .map(extract_plain_text)
            .collect::<Vec<_>>()
            .join("\n\n"),

        Node::Paragraph(para) => para
            .children
            .iter()
            .map(extract_plain_text)
            .collect::<Vec<_>>()
            .join(""),

        Node::Text(text) => text.value.clone(),

        Node::Heading(heading) => {
            let prefix = "#".repeat(heading.depth as usize);
            let text = heading
                .children
                .iter()
                .map(extract_plain_text)
                .collect::<Vec<_>>()
                .join("");
            format!("{} {}", prefix, text)
        }

        Node::Code(code) => format!("```\n{}\n```", code.value),

        Node::InlineCode(code) => format!("`{}`", code.value),

        Node::Link(link) => {
            let text = link
                .children
                .iter()
                .map(extract_plain_text)
                .collect::<Vec<_>>()
                .join("");
            format!("[{}]({})", text, link.url)
        }

        Node::List(list) => list
            .children
            .iter()
            .enumerate()
            .map(|(i, child)| {
                let bullet = if list.ordered {
                    format!("{}. ", i + 1)
                } else {
                    "- ".to_string()
                };
                format!("{}{}", bullet, extract_plain_text(child))
            })
            .collect::<Vec<_>>()
            .join("\n"),

        Node::ListItem(item) => item
            .children
            .iter()
            .map(extract_plain_text)
            .collect::<Vec<_>>()
            .join(""),

        Node::Blockquote(quote) => {
            let text = quote
                .children
                .iter()
                .map(extract_plain_text)
                .collect::<Vec<_>>()
                .join("\n");
            format!("> {}", text.replace('\n', "\n> "))
        }

        Node::Emphasis(emph) => {
            let text = emph
                .children
                .iter()
                .map(extract_plain_text)
                .collect::<Vec<_>>()
                .join("");
            format!("*{}*", text)
        }

        Node::Strong(strong) => {
            let text = strong
                .children
                .iter()
                .map(extract_plain_text)
                .collect::<Vec<_>>()
                .join("");
            format!("**{}**", text)
        }

        Node::Table(table) => table
            .children
            .iter()
            .map(extract_plain_text)
            .collect::<Vec<_>>()
            .join("\n"),

        Node::TableRow(row) => row
            .children
            .iter()
            .map(extract_plain_text)
            .collect::<Vec<_>>()
            .join(" | "),

        Node::TableCell(cell) => cell
            .children
            .iter()
            .map(extract_plain_text)
            .collect::<Vec<_>>()
            .join(""),

        // Fallback for other node types
        _ => String::new(),
    }
}

/// HTML to Markdown conversion errors
#[derive(Debug, Error)]
pub enum ConversionError {
    /// HTML too large
    #[error("HTML too large: {0} bytes (max 5MB per page)")]
    HtmlTooLarge(usize),

    /// HTML parsing error (from html_to_mdast)
    #[error("HTML parsing failed: {0}")]
    HtmlToMdastError(#[from] html_to_mdast::ConversionError),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test: Convert simple paragraph
    #[test]
    fn test_convert_simple_paragraph() {
        let html = "<p>Hello, world!</p>";
        let md = convert(html, 1).expect("Should convert");
        // mdast_util_to_markdown adds trailing newline (CommonMark standard)
        assert!(md.trim().starts_with("Hello, world!"));
    }

    /// Test: Convert headings
    #[test]
    fn test_convert_headings() {
        let html = "<h1>Title</h1><h2>Section</h2><h3>Subsection</h3>";
        let md = convert(html, 1).expect("Should convert");
        assert!(md.contains("# Title"));
        assert!(md.contains("## Section"));
        assert!(md.contains("### Subsection"));
    }

    /// Test: Convert inline code
    #[test]
    fn test_convert_inline_code() {
        let html = "<p>Use <code>print()</code> function</p>";
        let md = convert(html, 1).expect("Should convert");
        assert!(md.contains("`print()`"));
    }

    /// Test: Convert code block
    #[test]
    fn test_convert_code_block() {
        let html = "<pre><code>let x = 1;</code></pre>";
        let md = convert(html, 1).expect("Should convert");
        // mdast_util_to_markdown uses ``` without language by default
        assert!(md.contains("```"));
        assert!(md.contains("let x = 1;"));
    }

    /// Test: Convert Typst syntax highlighting (flattened to inline code)
    #[test]
    fn test_convert_typst_syntax() {
        let html =
            r#"<code><span class="typ-func">#image</span><span class="typ-punct">(</span></code>"#;
        let md = convert(html, 1).expect("Should convert");
        // Typst syntax spans are flattened to inline code
        assert!(md.contains("`#image(`") || md.contains("#image("));
    }

    /// Test: Inline code not in code block works correctly
    #[test]
    fn test_inline_code_outside_block() {
        let html = "<p>Use <code>func()</code> to call.</p>";
        let md = convert(html, 1).expect("Should convert");
        assert!(md.contains("`func()`"));
    }

    /// Test: HTML size limit
    #[test]
    fn test_size_limit() {
        let large_html = "x".repeat(MAX_HTML_SIZE + 1);
        let result = convert(&large_html, 1);
        assert!(result.is_err());
        match result.unwrap_err() {
            ConversionError::HtmlTooLarge(size) => {
                assert_eq!(size, MAX_HTML_SIZE + 1);
            }
            e => panic!("Expected HtmlTooLarge, got: {:?}", e),
        }
    }

    /// Test: Security - dangerous tags ignored
    #[test]
    fn test_dangerous_tags_ignored() {
        let html = r#"<p>Safe</p><script>alert("xss")</script><p>Also safe</p>"#;
        let md = convert(html, 1).expect("Should convert");
        assert!(md.contains("Safe"));
        assert!(md.contains("Also safe"));
        assert!(!md.contains("alert"));
        assert!(!md.contains("xss"));
    }

    /// Test: Security - style and link tags ignored
    #[test]
    fn test_style_and_link_tags_ignored() {
        let html = r#"<p>Content</p><style>.malicious { }</style><link rel="stylesheet" href="evil.css"><p>More content</p>"#;
        let md = convert(html, 1).expect("Should convert");
        assert!(md.contains("Content"));
        assert!(md.contains("More content"));
        assert!(!md.contains("malicious"));
        assert!(!md.contains("evil.css"));
    }

    /// Test: Empty HTML
    #[test]
    fn test_empty_html() {
        let html = "";
        let md = convert(html, 1).expect("Should convert");
        assert_eq!(md, "");
    }

    /// Test: Multiple paragraphs
    #[test]
    fn test_multiple_paragraphs() {
        let html = "<p>First paragraph.</p><p>Second paragraph.</p>";
        let md = convert(html, 1).expect("Should convert");
        assert!(md.contains("First paragraph."));
        assert!(md.contains("Second paragraph."));
    }
}
