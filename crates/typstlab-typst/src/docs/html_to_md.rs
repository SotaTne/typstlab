//! HTML to Markdown converter for Typst documentation
//!
//! Converts HTML content from docs.json to clean Markdown suitable for LLMs.
//! Handles Typst-specific HTML patterns (typ-* classes) and rustdoc compatibility.

use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever::Attribute;
use markup5ever::interface::QualName;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::cell::RefCell;
use thiserror::Error;

/// Maximum HTML size per page (5MB)
const MAX_HTML_SIZE: usize = 5_000_000;

/// Typst HTML to Markdown converter
///
/// Uses stack-based state management to correctly handle nested HTML structures.
pub struct TypstHtmlConverter {
    output: String,
    state_stack: Vec<ConverterMode>,
}

/// Converter state modes for nested context tracking
#[derive(Debug, Clone, PartialEq)]
enum ConverterMode {
    Normal,
    InCodeBlock,
    InTypstSpan { class: String },
}

impl TypstHtmlConverter {
    /// Creates a new converter instance
    fn new() -> Self {
        Self {
            output: String::new(),
            state_stack: vec![ConverterMode::Normal],
        }
    }

    /// Converts HTML to Markdown
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
    pub fn convert(html: &str) -> Result<String, ConversionError> {
        // Validate HTML size before parsing
        if html.len() > MAX_HTML_SIZE {
            return Err(ConversionError::HtmlTooLarge(html.len()));
        }

        // Parse HTML into DOM
        let dom = parse_document(RcDom::default(), Default::default())
            .from_utf8()
            .read_from(&mut html.as_bytes())
            .map_err(|e| ConversionError::ParseError(e.to_string()))?;

        // Walk DOM and convert to Markdown
        let mut converter = Self::new();
        converter.walk_node(&dom.document);

        Ok(converter.finalize())
    }

    /// Walks DOM tree recursively
    fn walk_node(&mut self, handle: &Handle) {
        match &handle.data {
            NodeData::Document => {
                // Document root - recurse into children
                for child in handle.children.borrow().iter() {
                    self.walk_node(child);
                }
            }
            NodeData::Element { name, attrs, .. } => {
                let tag = name.local.as_ref();

                // Skip dangerous tags entirely (don't process children)
                if matches!(tag, "script" | "iframe" | "object" | "embed") {
                    return;
                }

                // Enter element (push state if needed)
                self.handle_element_start(name, attrs);

                // Recurse children
                for child in handle.children.borrow().iter() {
                    self.walk_node(child);
                }

                // Exit element (pop state)
                self.handle_element_end(name, attrs);
            }
            NodeData::Text { contents } => {
                self.handle_text(&contents.borrow());
            }
            // Ignore comments, doctypes, etc.
            _ => {}
        }
    }

    /// Handles element start tag
    fn handle_element_start(&mut self, name: &QualName, attrs: &RefCell<Vec<Attribute>>) {
        let tag = name.local.as_ref();
        let class = self.get_class(attrs);

        match (tag, class.as_deref()) {
            // Headings
            ("h1", _) => self.output.push_str("\n# "),
            ("h2", _) => self.output.push_str("\n## "),
            ("h3", _) => self.output.push_str("\n### "),

            // Paragraphs
            ("p", _) => self.output.push_str("\n\n"),

            // Code blocks
            ("pre", _) => {
                self.output.push_str("\n```typ\n");
                self.state_stack.push(ConverterMode::InCodeBlock);
            }

            // Inline code (only if not in code block)
            ("code", _) if !self.in_code_block() => {
                self.output.push('`');
            }

            // Typst syntax spans
            ("span", Some(class)) if class.starts_with("typ-") => {
                self.state_stack.push(ConverterMode::InTypstSpan {
                    class: class.to_string(),
                });
            }

            _ => {}
        }
    }

    /// Handles element end tag
    fn handle_element_end(&mut self, name: &QualName, _attrs: &RefCell<Vec<Attribute>>) {
        let tag = name.local.as_ref();

        match tag {
            // Code blocks
            "pre" => {
                if self.in_code_block() {
                    self.output.push_str("\n```\n");
                    self.state_stack.pop();
                }
            }

            // Inline code
            "code" if !self.in_code_block() => {
                self.output.push('`');
            }

            // Typst spans
            "span" => {
                if matches!(
                    self.state_stack.last(),
                    Some(ConverterMode::InTypstSpan { .. })
                ) {
                    self.state_stack.pop();
                }
            }

            _ => {}
        }
    }

    /// Handles text nodes
    fn handle_text(&mut self, text: &str) {
        self.output.push_str(text);
    }

    /// Gets class attribute value
    fn get_class(&self, attrs: &RefCell<Vec<Attribute>>) -> Option<String> {
        attrs
            .borrow()
            .iter()
            .find(|attr| attr.name.local.as_ref() == "class")
            .map(|attr| attr.value.to_string())
    }

    /// Checks if currently in code block
    fn in_code_block(&self) -> bool {
        self.state_stack
            .iter()
            .any(|mode| matches!(mode, ConverterMode::InCodeBlock))
    }

    /// Finalizes conversion and returns Markdown
    fn finalize(self) -> String {
        self.output.trim().to_string()
    }
}

/// HTML to Markdown conversion errors
#[derive(Debug, Error)]
pub enum ConversionError {
    /// HTML too large
    #[error("HTML too large: {0} bytes (max 5MB per page)")]
    HtmlTooLarge(usize),

    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test: Convert simple paragraph
    #[test]
    fn test_convert_simple_paragraph() {
        let html = "<p>Hello, world!</p>";
        let md = TypstHtmlConverter::convert(html).expect("Should convert");
        assert_eq!(md, "Hello, world!");
    }

    /// Test: Convert headings
    #[test]
    fn test_convert_headings() {
        let html = "<h1>Title</h1><h2>Section</h2><h3>Subsection</h3>";
        let md = TypstHtmlConverter::convert(html).expect("Should convert");
        assert!(md.contains("# Title"));
        assert!(md.contains("## Section"));
        assert!(md.contains("### Subsection"));
    }

    /// Test: Convert inline code
    #[test]
    fn test_convert_inline_code() {
        let html = "<p>Use <code>print()</code> function</p>";
        let md = TypstHtmlConverter::convert(html).expect("Should convert");
        assert!(md.contains("`print()`"));
    }

    /// Test: Convert code block
    #[test]
    fn test_convert_code_block() {
        let html = "<pre><code>let x = 1;</code></pre>";
        let md = TypstHtmlConverter::convert(html).expect("Should convert");
        assert!(md.contains("```typ"));
        assert!(md.contains("let x = 1;"));
        assert!(md.contains("```"));
    }

    /// Test: Convert Typst syntax highlighting
    #[test]
    fn test_convert_typst_syntax() {
        let html =
            r#"<code><span class="typ-func">#image</span><span class="typ-punct">(</span></code>"#;
        let md = TypstHtmlConverter::convert(html).expect("Should convert");
        assert!(md.contains("`#image(`"));
    }

    /// Test: Inline code not in code block works correctly
    #[test]
    fn test_inline_code_outside_block() {
        let html = "<p>Use <code>func()</code> to call.</p>";
        let md = TypstHtmlConverter::convert(html).expect("Should convert");
        assert!(md.contains("`func()`"));
    }

    /// Test: HTML size limit
    #[test]
    fn test_size_limit() {
        let large_html = "x".repeat(MAX_HTML_SIZE + 1);
        let result = TypstHtmlConverter::convert(&large_html);
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
        let md = TypstHtmlConverter::convert(html).expect("Should convert");
        assert!(md.contains("Safe"));
        assert!(md.contains("Also safe"));
        assert!(!md.contains("alert"));
        assert!(!md.contains("xss"));
    }

    /// Test: Empty HTML
    #[test]
    fn test_empty_html() {
        let html = "";
        let md = TypstHtmlConverter::convert(html).expect("Should convert");
        assert_eq!(md, "");
    }

    /// Test: Multiple paragraphs
    #[test]
    fn test_multiple_paragraphs() {
        let html = "<p>First paragraph.</p><p>Second paragraph.</p>";
        let md = TypstHtmlConverter::convert(html).expect("Should convert");
        assert!(md.contains("First paragraph."));
        assert!(md.contains("Second paragraph."));
    }
}
