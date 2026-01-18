//! HTML to mdast converter for Typst documentation
//!
//! Converts HTML content from docs.json to mdast AST nodes for
//! subsequent Markdown generation via mdast_util_to_markdown.

use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markdown::mdast::{AlignKind, Node};
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
struct TypstHtmlConverter {
    /// Root children accumulator
    root_children: Vec<Node>,
    /// Current paragraph accumulator
    current_paragraph: Option<Vec<Node>>,
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

    /// Handles heading elements (h1-h6)
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
        let heading_node = self.build_heading(depth, handle);
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
        let code_text = self.collect_text_from_children(handle);
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
        let href = self.get_attr(attrs, "href");
        let link_node = self.build_link(href, handle);

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
        let list_node = self.build_list(ordered, handle);
        self.root_children.push(list_node);
    }

    /// Handles blockquote element
    fn handle_blockquote_element(&mut self, handle: &Handle) {
        self.end_paragraph();
        let blockquote_node = self.build_blockquote(handle);
        self.root_children.push(blockquote_node);
    }

    /// Handles inline code element (<code>)
    fn handle_inline_code(&mut self, handle: &Handle) {
        let code_text = self.collect_text_from_children(handle);
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
        let emphasis_node = self.build_emphasis(handle);

        if self.current_paragraph.is_none() {
            self.start_paragraph();
        }
        if let Some(para) = &mut self.current_paragraph {
            para.push(emphasis_node);
        }
    }

    /// Handles strong element (<strong> or <b>)
    fn handle_strong_element(&mut self, handle: &Handle) {
        let strong_node = self.build_strong(handle);

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
        let table_node = self.build_table(handle);
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

    /// Gets class attribute value
    #[allow(dead_code)]
    fn get_class(&self, attrs: &RefCell<Vec<Attribute>>) -> Option<String> {
        attrs
            .borrow()
            .iter()
            .find(|attr| attr.name.local.as_ref() == "class")
            .map(|attr| attr.value.to_string())
    }

    /// Gets attribute value by name
    fn get_attr(&self, attrs: &RefCell<Vec<Attribute>>, name: &str) -> Option<String> {
        attrs
            .borrow()
            .iter()
            .find(|attr| attr.name.local.as_ref() == name)
            .map(|attr| attr.value.to_string())
    }

    /// Builds Heading node from <h1> - <h6>
    fn build_heading(&mut self, depth: u8, handle: &Handle) -> Node {
        let saved_para = self.current_paragraph.take();

        // Temporarily accumulate children inline
        self.current_paragraph = Some(Vec::new());
        for child in handle.children.borrow().iter() {
            self.walk_node(child);
        }

        // Extract accumulated children
        let children = self.current_paragraph.take().unwrap_or_default();
        self.current_paragraph = saved_para;

        Node::Heading(markdown::mdast::Heading {
            children,
            depth,
            position: None,
        })
    }

    /// Builds Link node from <a> element
    fn build_link(&mut self, href: Option<String>, handle: &Handle) -> Node {
        // Collect link text children
        let mut link_children = Vec::new();
        let saved_para = self.current_paragraph.take();

        // Temporarily accumulate children inline
        self.current_paragraph = Some(Vec::new());
        for child in handle.children.borrow().iter() {
            self.walk_node(child);
        }

        // Extract accumulated children
        if let Some(children) = self.current_paragraph.take() {
            link_children = children;
        }

        // Restore previous paragraph state
        self.current_paragraph = saved_para;

        // Fix internal links: /DOCS-BASE/ → ../
        let url = href
            .unwrap_or_else(|| "#".to_string())
            .replace("/DOCS-BASE/", "../");

        Node::Link(markdown::mdast::Link {
            children: link_children,
            url,
            title: None,
            position: None,
        })
    }

    /// Builds List node from <ul> or <ol>
    fn build_list(&mut self, ordered: bool, handle: &Handle) -> Node {
        let mut items = Vec::new();

        for child in handle.children.borrow().iter() {
            if let NodeData::Element { name, .. } = &child.data
                && name.local.as_ref() == "li"
            {
                items.push(self.build_list_item(child));
            }
        }

        Node::List(markdown::mdast::List {
            children: items,
            ordered,
            start: if ordered { Some(1) } else { None },
            spread: false,
            position: None,
        })
    }

    /// Builds ListItem node from <li>
    fn build_list_item(&mut self, handle: &Handle) -> Node {
        let saved_root = std::mem::take(&mut self.root_children);
        let saved_para = self.current_paragraph.take();

        // Process children (may contain paragraphs, nested lists)
        for child in handle.children.borrow().iter() {
            self.walk_node(child);
        }

        // Flush any remaining paragraph
        self.end_paragraph();

        // Extract accumulated children
        let item_children = std::mem::replace(&mut self.root_children, saved_root);
        self.current_paragraph = saved_para;

        Node::ListItem(markdown::mdast::ListItem {
            children: item_children,
            checked: None,
            spread: false,
            position: None,
        })
    }

    /// Builds Blockquote node from <blockquote>
    fn build_blockquote(&mut self, handle: &Handle) -> Node {
        let saved_root = std::mem::take(&mut self.root_children);

        // Process children
        for child in handle.children.borrow().iter() {
            self.walk_node(child);
        }

        // Flush any remaining paragraph
        self.end_paragraph();

        // Extract accumulated children
        let blockquote_children = std::mem::replace(&mut self.root_children, saved_root);

        Node::Blockquote(markdown::mdast::Blockquote {
            children: blockquote_children,
            position: None,
        })
    }

    /// Collects all text from children recursively
    fn collect_text_from_children(&self, handle: &Handle) -> String {
        let mut text = String::new();
        for child in handle.children.borrow().iter() {
            match &child.data {
                NodeData::Text { contents } => {
                    text.push_str(&contents.borrow());
                }
                NodeData::Element { .. } => {
                    text.push_str(&self.collect_text_from_children(child));
                }
                _ => {}
            }
        }
        text
    }

    /// Builds Emphasis node from <em> or <i>
    fn build_emphasis(&mut self, handle: &Handle) -> Node {
        let saved_para = self.current_paragraph.take();

        self.current_paragraph = Some(Vec::new());
        for child in handle.children.borrow().iter() {
            self.walk_node(child);
        }

        let children = self.current_paragraph.take().unwrap_or_default();
        self.current_paragraph = saved_para;

        Node::Emphasis(markdown::mdast::Emphasis {
            children,
            position: None,
        })
    }

    /// Builds Strong node from <strong> or <b>
    fn build_strong(&mut self, handle: &Handle) -> Node {
        let saved_para = self.current_paragraph.take();

        self.current_paragraph = Some(Vec::new());
        for child in handle.children.borrow().iter() {
            self.walk_node(child);
        }

        let children = self.current_paragraph.take().unwrap_or_default();
        self.current_paragraph = saved_para;

        Node::Strong(markdown::mdast::Strong {
            children,
            position: None,
        })
    }

    /// Builds Table node from <table>
    fn build_table(&mut self, handle: &Handle) -> Node {
        let mut rows = Vec::new();
        let mut col_count = 0;

        // Process all rows (from thead and tbody)
        for child in handle.children.borrow().iter() {
            if let NodeData::Element { name, .. } = &child.data {
                let tag = name.local.as_ref();
                if tag == "thead" || tag == "tbody" {
                    // Process rows within thead/tbody
                    for row_child in child.children.borrow().iter() {
                        if let NodeData::Element { name, .. } = &row_child.data
                            && name.local.as_ref() == "tr"
                        {
                            let row = self.build_table_row(row_child);
                            if let Node::TableRow(ref r) = row {
                                col_count = col_count.max(r.children.len());
                            }
                            rows.push(row);
                        }
                    }
                } else if tag == "tr" {
                    // Direct <tr> (no thead/tbody wrapper)
                    let row = self.build_table_row(child);
                    if let Node::TableRow(ref r) = row {
                        col_count = col_count.max(r.children.len());
                    }
                    rows.push(row);
                }
            }
        }

        // Default alignment: none (left-aligned)
        let align = vec![AlignKind::None; col_count];

        Node::Table(markdown::mdast::Table {
            children: rows,
            align,
            position: None,
        })
    }

    /// Builds TableRow node from <tr>
    fn build_table_row(&mut self, handle: &Handle) -> Node {
        let mut cells = Vec::new();

        for child in handle.children.borrow().iter() {
            if let NodeData::Element { name, .. } = &child.data {
                let tag = name.local.as_ref();
                if tag == "th" || tag == "td" {
                    cells.push(self.build_table_cell(child));
                }
            }
        }

        Node::TableRow(markdown::mdast::TableRow {
            children: cells,
            position: None,
        })
    }

    /// Builds TableCell node from <th> or <td>
    fn build_table_cell(&mut self, handle: &Handle) -> Node {
        let saved_root = std::mem::take(&mut self.root_children);
        let saved_para = self.current_paragraph.take();

        // Process cell content
        for child in handle.children.borrow().iter() {
            self.walk_node(child);
        }

        // Flush any remaining paragraph
        self.end_paragraph();

        // Extract accumulated children
        let cell_children = std::mem::replace(&mut self.root_children, saved_root);
        self.current_paragraph = saved_para;

        // If no children, add empty text node
        let children = if cell_children.is_empty() {
            vec![Node::Text(markdown::mdast::Text {
                value: String::new(),
                position: None,
            })]
        } else {
            cell_children
        };

        Node::TableCell(markdown::mdast::TableCell {
            children,
            position: None,
        })
    }

    /// Start paragraph accumulator
    fn start_paragraph(&mut self) {
        if self.current_paragraph.is_none() {
            self.current_paragraph = Some(Vec::new());
        }
    }

    /// End paragraph and flush to root
    fn end_paragraph(&mut self) {
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
                    // URL should be rewritten: /DOCS-BASE/ → ../
                    assert_eq!(
                        link.url, "../reference/func",
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
                assert!(!list.ordered, "Should be unordered");
                assert_eq!(list.children.len(), 2, "Should have two items");

                // Verify first item
                if let Node::ListItem(item) = &list.children[0] {
                    assert_eq!(item.children.len(), 1, "Item should have paragraph");
                    if let Node::Paragraph(para) = &item.children[0]
                        && let Node::Text(text) = &para.children[0]
                    {
                        assert_eq!(text.value, "Item 1");
                    }
                }
            } else {
                panic!("Root should contain List node");
            }
        }

        // Test ordered list
        let html_ol = r#"<ol><li>First</li><li>Second</li></ol>"#;

        let result_ol = convert(html_ol).expect("Should convert ordered list");

        if let Node::Root(root) = result_ol
            && let Node::List(list) = &root.children[0]
        {
            assert!(list.ordered, "Should be ordered");
            assert_eq!(list.start, Some(1), "Should start at 1");
        }
    }

    /// Test: Convert <table> to mdast Table node
    ///
    /// Codex Requirement 1 - Mandatory Test 3/5
    #[test]
    fn test_table_conversion() {
        let html = r#"
            <table>
                <thead>
                    <tr><th>Name</th><th>Type</th></tr>
                </thead>
                <tbody>
                    <tr><td>param1</td><td>string</td></tr>
                    <tr><td>param2</td><td>int</td></tr>
                </tbody>
            </table>
        "#;

        let result = convert(html).expect("Should convert table");

        if let Node::Root(root) = result {
            assert_eq!(root.children.len(), 1, "Should have one table");

            if let Node::Table(table) = &root.children[0] {
                // Header row + 2 body rows = 3 rows
                assert_eq!(table.children.len(), 3, "Should have 3 rows");

                // Verify alignment (default: none)
                assert_eq!(table.align.len(), 2, "Should have 2 columns");

                // Verify header row
                if let Node::TableRow(row) = &table.children[0] {
                    assert_eq!(row.children.len(), 2, "Header should have 2 cells");
                    if let Node::TableCell(cell) = &row.children[0]
                        && let Node::Text(text) = &cell.children[0]
                    {
                        assert_eq!(text.value, "Name");
                    }
                }

                // Verify data row
                if let Node::TableRow(row) = &table.children[1]
                    && let Node::TableCell(cell) = &row.children[0]
                    && let Node::Text(text) = &cell.children[0]
                {
                    assert_eq!(text.value, "param1");
                }
            } else {
                panic!("Root should contain Table node");
            }
        }
    }

    /// Test: Convert <blockquote> to mdast Blockquote node
    ///
    /// Codex Requirement 1 - Mandatory Test 4/5
    #[test]
    fn test_blockquote_conversion() {
        let html = r#"<blockquote><p>Important note about this function.</p></blockquote>"#;

        let result = convert(html).expect("Should convert blockquote");

        if let Node::Root(root) = result {
            assert_eq!(root.children.len(), 1, "Should have one blockquote");

            if let Node::Blockquote(blockquote) = &root.children[0] {
                assert_eq!(
                    blockquote.children.len(),
                    1,
                    "Blockquote should have paragraph"
                );

                if let Node::Paragraph(para) = &blockquote.children[0] {
                    if let Node::Text(text) = &para.children[0] {
                        assert_eq!(text.value, "Important note about this function.");
                    }
                } else {
                    panic!("Blockquote should contain Paragraph");
                }
            } else {
                panic!("Root should contain Blockquote node");
            }
        }
    }

    /// Test: Convert nested structures (list containing code)
    ///
    /// Codex Requirement 1 - Mandatory Test 5/5
    #[test]
    fn test_nested_structures() {
        let html = r#"
            <ul>
                <li>Use <code>function()</code> to call</li>
                <li>Nested list:
                    <ul>
                        <li>Inner item</li>
                    </ul>
                </li>
            </ul>
        "#;

        let result = convert(html).expect("Should convert nested structure");

        if let Node::Root(root) = result
            && let Node::List(list) = &root.children[0]
        {
            assert_eq!(list.children.len(), 2, "Should have 2 items");

            // First item: paragraph with inline code
            if let Node::ListItem(item) = &list.children[0]
                && let Node::Paragraph(para) = &item.children[0]
            {
                // Should contain: Text("Use ") + InlineCode("function()") + Text(" to call")
                assert!(para.children.len() >= 2, "Should have mixed content");

                // Find InlineCode node
                let has_code = para.children.iter().any(|node| {
                    if let Node::InlineCode(code) = node {
                        code.value == "function()"
                    } else {
                        false
                    }
                });
                assert!(has_code, "Should contain InlineCode node");
            }

            // Second item: nested list
            if let Node::ListItem(item) = &list.children[1] {
                // Should contain paragraph + nested list
                let has_nested_list = item
                    .children
                    .iter()
                    .any(|node| matches!(node, Node::List(_)));
                assert!(has_nested_list, "Should contain nested List");
            }
        }
    }

    /// Test: Typst syntax spans flattened to inline code
    ///
    /// Additional test for Typst-specific HTML patterns
    #[test]
    fn test_typst_syntax_spans_flattened() {
        let html = r#"<code><span class="typ-func">#image</span><span class="typ-punct">(</span><span class="typ-str">"file.jpg"</span><span class="typ-punct">)</span></code>"#;

        let result = convert(html).expect("Should convert Typst syntax");

        if let Node::Root(root) = result
            && let Node::Paragraph(para) = &root.children[0]
        {
            // Typst spans should be flattened to single InlineCode
            if let Node::InlineCode(code) = &para.children[0] {
                // Should preserve all text, lose highlighting
                assert!(code.value.contains("#image"));
                assert!(code.value.contains("file.jpg"));
            } else {
                panic!("Should contain InlineCode node");
            }
        }
    }
}
