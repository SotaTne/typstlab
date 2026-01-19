//! HTML to mdast converter for Typst documentation
//!
//! Converts HTML content from docs.json to mdast AST nodes for
//! subsequent Markdown generation via mdast_util_to_markdown.

mod builders;
mod helpers;

use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markdown::mdast::Node;
use markup5ever::{Attribute, QualName};
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::cell::RefCell;
use thiserror::Error;

/// Converts HTML string to mdast Root node
///
/// # Arguments
///
/// * `html` - HTML string to convert
///
/// # Returns
///
/// mdast Root node containing the converted structure
///
/// # Errors
///
/// Returns error if HTML parsing fails
pub fn convert(html: &str) -> Result<Node, ConversionError> {
    // Parse HTML into DOM
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .map_err(|e| ConversionError::ParseError(e.to_string()))?;

    // Walk DOM and convert to mdast
    let mut converter = TypstHtmlConverter::new();
    converter.walk_node(&dom.document);

    Ok(converter.finalize())
}

/// Typst HTML to mdast converter (internal implementation)
pub(super) struct TypstHtmlConverter {
    /// Root children accumulator
    pub(super) root_children: Vec<Node>,
    /// Current paragraph accumulator
    pub(super) current_paragraph: Option<Vec<Node>>,
}

impl TypstHtmlConverter {
    /// Creates a new converter instance
    fn new() -> Self {
        Self {
            root_children: Vec::new(),
            current_paragraph: None,
        }
    }

    /// Walks DOM tree recursively
    pub(super) fn walk_node(&mut self, handle: &Handle) {
        match &handle.data {
            NodeData::Document => {
                // Document root - recurse into children
                for child in handle.children.borrow().iter() {
                    self.walk_node(child);
                }
            }
            NodeData::Element { name, attrs, .. } => {
                let tag = name.local.as_ref();

                // Skip dangerous/irrelevant tags entirely
                if matches!(
                    tag,
                    "script" | "iframe" | "object" | "embed" | "style" | "link"
                ) {
                    return;
                }

                // Handle element
                self.handle_element_start(name, attrs, handle);
            }
            NodeData::Text { contents } => {
                self.handle_text(&contents.borrow());
            }
            _ => {}
        }
    }

    /// Handles element start tag and dispatches to specific handlers
    fn handle_element_start(
        &mut self,
        name: &QualName,
        attrs: &RefCell<Vec<Attribute>>,
        handle: &Handle,
    ) {
        let tag = name.local.as_ref();

        match tag {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.handle_heading(tag, handle);
            }
            "p" => {
                self.handle_paragraph(handle);
            }
            "pre" => {
                self.handle_code_block(handle);
            }
            "a" => {
                self.handle_link(attrs, handle);
            }
            "ul" => {
                self.handle_list(false, handle);
            }
            "ol" => {
                self.handle_list(true, handle);
            }
            "blockquote" => {
                self.handle_blockquote_element(handle);
            }
            "code" => {
                self.handle_inline_code(handle);
            }
            "em" | "i" => {
                self.handle_emphasis_element(handle);
            }
            "strong" | "b" => {
                self.handle_strong_element(handle);
            }
            "table" => {
                self.handle_table_element(handle);
            }
            _ => {
                self.handle_default(handle);
            }
        }
    }

    /// Handles heading element (<h1> - <h6>)
    fn handle_heading(&mut self, tag: &str, handle: &Handle) {
        self.end_paragraph();
        let depth = match tag {
            "h1" => 1,
            "h2" => 2,
            "h3" => 3,
            "h4" => 4,
            "h5" => 5,
            "h6" => 6,
            _ => 1,
        };
        let heading_node = builders::build_heading(self, depth, handle);
        self.root_children.push(heading_node);
    }

    /// Handles paragraph element
    fn handle_paragraph(&mut self, handle: &Handle) {
        self.start_paragraph();
        for child in handle.children.borrow().iter() {
            self.walk_node(child);
        }
        self.end_paragraph();
    }

    /// Handles code block element (<pre>)
    fn handle_code_block(&mut self, handle: &Handle) {
        self.end_paragraph();
        let code_text = helpers::collect_text_from_children(handle);
        let code_node = Node::Code(markdown::mdast::Code {
            value: code_text,
            lang: None,
            meta: None,
            position: None,
        });
        self.root_children.push(code_node);
    }

    /// Handles link element (<a>)
    fn handle_link(&mut self, attrs: &RefCell<Vec<Attribute>>, handle: &Handle) {
        let href = helpers::get_attr(attrs, "href");
        let link_node = builders::build_link(self, href, handle);

        if self.current_paragraph.is_none() {
            self.start_paragraph();
        }
        if let Some(para) = &mut self.current_paragraph {
            para.push(link_node);
        }
    }

    /// Handles list element (<ul> or <ol>)
    fn handle_list(&mut self, ordered: bool, handle: &Handle) {
        self.end_paragraph();
        let list_node = builders::build_list(self, ordered, handle);
        self.root_children.push(list_node);
    }

    /// Handles blockquote element
    fn handle_blockquote_element(&mut self, handle: &Handle) {
        self.end_paragraph();
        let blockquote_node = builders::build_blockquote(self, handle);
        self.root_children.push(blockquote_node);
    }

    /// Handles inline code element (<code>)
    fn handle_inline_code(&mut self, handle: &Handle) {
        let code_text = helpers::collect_text_from_children(handle);
        let code_node = Node::InlineCode(markdown::mdast::InlineCode {
            value: code_text,
            position: None,
        });

        if self.current_paragraph.is_none() {
            self.start_paragraph();
        }
        if let Some(para) = &mut self.current_paragraph {
            para.push(code_node);
        }
    }

    /// Handles emphasis element (<em> or <i>)
    fn handle_emphasis_element(&mut self, handle: &Handle) {
        let emphasis_node = builders::build_emphasis(self, handle);

        if self.current_paragraph.is_none() {
            self.start_paragraph();
        }
        if let Some(para) = &mut self.current_paragraph {
            para.push(emphasis_node);
        }
    }

    /// Handles strong element (<strong> or <b>)
    fn handle_strong_element(&mut self, handle: &Handle) {
        let strong_node = builders::build_strong(self, handle);

        if self.current_paragraph.is_none() {
            self.start_paragraph();
        }
        if let Some(para) = &mut self.current_paragraph {
            para.push(strong_node);
        }
    }

    /// Handles table element
    fn handle_table_element(&mut self, handle: &Handle) {
        self.end_paragraph();
        let table_node = builders::build_table(self, handle);
        self.root_children.push(table_node);
    }

    /// Handles default/unknown elements (recurse into children)
    fn handle_default(&mut self, handle: &Handle) {
        for child in handle.children.borrow().iter() {
            self.walk_node(child);
        }
    }

    /// Handles text nodes
    fn handle_text(&mut self, text: &str) {
        if text.trim().is_empty() {
            return;
        }

        let text_node = Node::Text(markdown::mdast::Text {
            value: text.to_string(),
            position: None,
        });

        // Add to current paragraph or root
        if let Some(para) = &mut self.current_paragraph {
            para.push(text_node);
        } else {
            // Auto-wrap orphan text in paragraph
            self.start_paragraph();
            if let Some(para) = &mut self.current_paragraph {
                para.push(text_node);
            }
        }
    }

    /// Start paragraph accumulator
    fn start_paragraph(&mut self) {
        if self.current_paragraph.is_none() {
            self.current_paragraph = Some(Vec::new());
        }
    }

    /// End paragraph and flush to root
    pub(super) fn end_paragraph(&mut self) {
        if let Some(children) = self.current_paragraph.take()
            && !children.is_empty()
        {
            self.root_children
                .push(Node::Paragraph(markdown::mdast::Paragraph {
                    children,
                    position: None,
                }));
        }
    }

    /// Accumulate inline children temporarily
    ///
    /// Saves current paragraph state, processes children into temporary buffer,
    /// returns accumulated nodes, and restores previous state.
    ///
    /// # Arguments
    ///
    /// * `handle` - DOM node whose children to process
    ///
    /// # Returns
    ///
    /// Vec of accumulated inline nodes
    pub(super) fn accumulate_inline_children(&mut self, handle: &Handle) -> Vec<Node> {
        let saved_para = self.current_paragraph.take();

        // Temporarily accumulate children inline
        self.current_paragraph = Some(Vec::new());
        for child in handle.children.borrow().iter() {
            self.walk_node(child);
        }

        // Extract accumulated children
        let children = self.current_paragraph.take().unwrap_or_default();
        self.current_paragraph = saved_para;

        children
    }

    /// Execute closure with saved root_children
    ///
    /// Saves root_children, executes closure (which may modify root_children),
    /// extracts accumulated children, and restores previous root_children.
    ///
    /// # Arguments
    ///
    /// * `f` - Closure to execute
    ///
    /// # Returns
    ///
    /// Vec of nodes accumulated in root_children during closure execution
    pub(super) fn with_saved_root_children<F>(&mut self, f: F) -> Vec<Node>
    where
        F: FnOnce(&mut Self),
    {
        let saved_root = std::mem::take(&mut self.root_children);

        f(self);

        // Extract accumulated children
        std::mem::replace(&mut self.root_children, saved_root)
    }

    /// Finalizes conversion and returns mdast Root
    fn finalize(mut self) -> Node {
        // Flush any remaining paragraph
        self.end_paragraph();

        Node::Root(markdown::mdast::Root {
            children: self.root_children,
            position: None,
        })
    }
}

/// HTML to mdast conversion errors
#[derive(Debug, Error)]
pub enum ConversionError {
    /// HTML parsing error
    #[error("HTML parsing failed: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test: Convert <a> tag to mdast Link node
    ///
    /// Codex Requirement 1 - Mandatory Test 1/5
    #[test]
    fn test_link_conversion() {
        let html = r#"<a href="/DOCS-BASE/reference/func">Function Reference</a>"#;

        let result = convert(html).expect("Should convert link");

        // Verify Root node
        if let Node::Root(root) = result {
            assert_eq!(root.children.len(), 1, "Should have one paragraph");

            // Verify Paragraph containing Link
            if let Node::Paragraph(para) = &root.children[0] {
                assert_eq!(para.children.len(), 1, "Should have one link");

                // Verify Link node
                if let Node::Link(link) = &para.children[0] {
                    // URL should be rewritten: /DOCS-BASE/ → .md format
                    assert_eq!(
                        link.url, "../reference/func.md",
                        "Should rewrite internal links"
                    );

                    // Verify link text
                    assert_eq!(link.children.len(), 1, "Should have one text child");
                    if let Node::Text(text) = &link.children[0] {
                        assert_eq!(text.value, "Function Reference");
                    } else {
                        panic!("Link should contain Text node");
                    }
                } else {
                    panic!("Paragraph should contain Link node");
                }
            } else {
                panic!("Root should contain Paragraph node");
            }
        } else {
            panic!("Result should be Root node");
        }
    }

    /// Test: Convert <ul> and <ol> to mdast List nodes
    ///
    /// Codex Requirement 1 - Mandatory Test 2/5
    #[test]
    fn test_list_conversion() {
        // Test unordered list
        let html_ul = r#"<ul><li>Item 1</li><li>Item 2</li></ul>"#;

        let result_ul = convert(html_ul).expect("Should convert unordered list");

        if let Node::Root(root) = result_ul {
            assert_eq!(root.children.len(), 1, "Should have one list");

            if let Node::List(list) = &root.children[0] {
                assert!(!list.ordered, "Should be unordered list");
                assert_eq!(list.children.len(), 2, "Should have two items");
            } else {
                panic!("Root should contain List node");
            }
        } else {
            panic!("Result should be Root node");
        }

        // Test ordered list
        let html_ol = r#"<ol><li>First</li><li>Second</li></ol>"#;

        let result_ol = convert(html_ol).expect("Should convert ordered list");

        if let Node::Root(root) = result_ol {
            if let Node::List(list) = &root.children[0] {
                assert!(list.ordered, "Should be ordered list");
                assert_eq!(list.start, Some(1), "Should start at 1");
            } else {
                panic!("Root should contain List node");
            }
        } else {
            panic!("Result should be Root node");
        }
    }

    /// Test: Convert <table> to mdast Table node
    ///
    /// Codex Requirement 1 - Mandatory Test 3/5
    #[test]
    fn test_table_conversion() {
        let html = r#"<table>
            <thead><tr><th>Name</th><th>Value</th></tr></thead>
            <tbody><tr><td>Alpha</td><td>1</td></tr></tbody>
        </table>"#;

        let result = convert(html).expect("Should convert table");

        if let Node::Root(root) = result {
            assert_eq!(root.children.len(), 1, "Should have one table");

            if let Node::Table(table) = &root.children[0] {
                assert_eq!(table.children.len(), 2, "Should have two rows");
                assert_eq!(table.align.len(), 2, "Should have alignment for 2 columns");
            } else {
                panic!("Root should contain Table node");
            }
        } else {
            panic!("Result should be Root node");
        }
    }

    /// Test: Convert <blockquote> to mdast Blockquote node
    ///
    /// Codex Requirement 1 - Mandatory Test 4/5
    #[test]
    fn test_blockquote_conversion() {
        let html = r#"<blockquote><p>Important note</p></blockquote>"#;

        let result = convert(html).expect("Should convert blockquote");

        if let Node::Root(root) = result {
            assert_eq!(root.children.len(), 1, "Should have one blockquote");

            if let Node::Blockquote(quote) = &root.children[0] {
                assert_eq!(quote.children.len(), 1, "Should have one paragraph inside");

                if let Node::Paragraph(para) = &quote.children[0] {
                    if let Node::Text(text) = &para.children[0] {
                        assert_eq!(text.value, "Important note");
                    } else {
                        panic!("Paragraph should contain Text node");
                    }
                } else {
                    panic!("Blockquote should contain Paragraph node");
                }
            } else {
                panic!("Root should contain Blockquote node");
            }
        } else {
            panic!("Result should be Root node");
        }
    }

    /// Test: Nested structures (list inside code, etc.)
    ///
    /// Codex Requirement 1 - Mandatory Test 5/5
    #[test]
    fn test_nested_structures() {
        let html = r#"<ul><li><code>function()</code> calls</li><li>Returns <strong>result</strong></li></ul>"#;

        let result = convert(html).expect("Should convert nested structures");

        if let Node::Root(root) = result {
            assert_eq!(root.children.len(), 1, "Should have one list");

            if let Node::List(list) = &root.children[0] {
                assert_eq!(list.children.len(), 2, "Should have two items");

                // First item: inline code inside paragraph
                if let Node::ListItem(item1) = &list.children[0] {
                    if let Node::Paragraph(para1) = &item1.children[0] {
                        // Should have InlineCode and Text nodes
                        assert!(para1.children.len() >= 2, "Should have multiple children");

                        if let Node::InlineCode(code) = &para1.children[0] {
                            assert_eq!(code.value, "function()");
                        } else {
                            panic!("First child should be InlineCode");
                        }
                    } else {
                        panic!("ListItem should contain Paragraph");
                    }
                } else {
                    panic!("List should contain ListItem");
                }

                // Second item: strong inside paragraph
                if let Node::ListItem(item2) = &list.children[1] {
                    if let Node::Paragraph(para2) = &item2.children[0] {
                        // Should have Text, Strong, etc.
                        assert!(para2.children.len() >= 2, "Should have multiple children");
                    } else {
                        panic!("ListItem should contain Paragraph");
                    }
                } else {
                    panic!("List should contain ListItem");
                }
            } else {
                panic!("Root should contain List node");
            }
        } else {
            panic!("Result should be Root node");
        }
    }
}
