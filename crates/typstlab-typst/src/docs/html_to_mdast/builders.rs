//! Builder functions for mdast nodes
//!
//! These functions construct mdast nodes from HTML elements.
//! They are called by TypstHtmlConverter during DOM traversal.

use markdown::mdast::{
    AlignKind, Blockquote, Emphasis, Heading, Link, List, ListItem, Node, Strong, Table, TableCell,
    TableRow, Text,
};
use markup5ever_rcdom::{Handle, NodeData};

use super::TypstHtmlConverter;

/// Builds Heading node from <h1> - <h6>
pub(super) fn build_heading(
    converter: &mut TypstHtmlConverter,
    depth: u8,
    handle: &Handle,
) -> Node {
    let saved_para = converter.current_paragraph.take();

    // Temporarily accumulate children inline
    converter.current_paragraph = Some(Vec::new());
    for child in handle.children.borrow().iter() {
        converter.walk_node(child);
    }

    // Extract accumulated children
    let children = converter.current_paragraph.take().unwrap_or_default();
    converter.current_paragraph = saved_para;

    Node::Heading(Heading {
        children,
        depth,
        position: None,
    })
}

/// Builds Link node from <a> element
pub(super) fn build_link(
    converter: &mut TypstHtmlConverter,
    href: Option<String>,
    handle: &Handle,
) -> Node {
    // Collect link text children
    let mut link_children = Vec::new();
    let saved_para = converter.current_paragraph.take();

    // Temporarily accumulate children inline
    converter.current_paragraph = Some(Vec::new());
    for child in handle.children.borrow().iter() {
        converter.walk_node(child);
    }

    // Extract accumulated children
    if let Some(children) = converter.current_paragraph.take() {
        link_children = children;
    }

    // Restore previous paragraph state
    converter.current_paragraph = saved_para;

    // Fix internal links: /DOCS-BASE/ → ../
    let url = href
        .unwrap_or_else(|| "#".to_string())
        .replace("/DOCS-BASE/", "../");

    Node::Link(Link {
        children: link_children,
        url,
        title: None,
        position: None,
    })
}

/// Builds List node from <ul> or <ol>
pub(super) fn build_list(
    converter: &mut TypstHtmlConverter,
    ordered: bool,
    handle: &Handle,
) -> Node {
    let mut items = Vec::new();

    for child in handle.children.borrow().iter() {
        if let NodeData::Element { name, .. } = &child.data
            && name.local.as_ref() == "li"
        {
            items.push(build_list_item(converter, child));
        }
    }

    Node::List(List {
        children: items,
        ordered,
        start: if ordered { Some(1) } else { None },
        spread: false,
        position: None,
    })
}

/// Builds ListItem node from <li>
pub(super) fn build_list_item(converter: &mut TypstHtmlConverter, handle: &Handle) -> Node {
    let saved_root = std::mem::take(&mut converter.root_children);
    let saved_para = converter.current_paragraph.take();

    // Process children (may contain paragraphs, nested lists)
    for child in handle.children.borrow().iter() {
        converter.walk_node(child);
    }

    // Flush any remaining paragraph
    converter.end_paragraph();

    // Extract accumulated children
    let item_children = std::mem::replace(&mut converter.root_children, saved_root);
    converter.current_paragraph = saved_para;

    Node::ListItem(ListItem {
        children: item_children,
        checked: None,
        spread: false,
        position: None,
    })
}

/// Builds Blockquote node from <blockquote>
pub(super) fn build_blockquote(converter: &mut TypstHtmlConverter, handle: &Handle) -> Node {
    let saved_root = std::mem::take(&mut converter.root_children);

    // Process children
    for child in handle.children.borrow().iter() {
        converter.walk_node(child);
    }

    // Flush any remaining paragraph
    converter.end_paragraph();

    // Extract accumulated children
    let blockquote_children = std::mem::replace(&mut converter.root_children, saved_root);

    Node::Blockquote(Blockquote {
        children: blockquote_children,
        position: None,
    })
}

/// Builds Emphasis node from <em> or <i>
pub(super) fn build_emphasis(converter: &mut TypstHtmlConverter, handle: &Handle) -> Node {
    let saved_para = converter.current_paragraph.take();

    converter.current_paragraph = Some(Vec::new());
    for child in handle.children.borrow().iter() {
        converter.walk_node(child);
    }

    let children = converter.current_paragraph.take().unwrap_or_default();
    converter.current_paragraph = saved_para;

    Node::Emphasis(Emphasis {
        children,
        position: None,
    })
}

/// Builds Strong node from <strong> or <b>
pub(super) fn build_strong(converter: &mut TypstHtmlConverter, handle: &Handle) -> Node {
    let saved_para = converter.current_paragraph.take();

    converter.current_paragraph = Some(Vec::new());
    for child in handle.children.borrow().iter() {
        converter.walk_node(child);
    }

    let children = converter.current_paragraph.take().unwrap_or_default();
    converter.current_paragraph = saved_para;

    Node::Strong(Strong {
        children,
        position: None,
    })
}

/// Builds Table node from <table>
pub(super) fn build_table(converter: &mut TypstHtmlConverter, handle: &Handle) -> Node {
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
                        let row = build_table_row(converter, row_child);
                        if let Node::TableRow(ref r) = row {
                            col_count = col_count.max(r.children.len());
                        }
                        rows.push(row);
                    }
                }
            } else if tag == "tr" {
                // Direct <tr> (no thead/tbody wrapper)
                let row = build_table_row(converter, child);
                if let Node::TableRow(ref r) = row {
                    col_count = col_count.max(r.children.len());
                }
                rows.push(row);
            }
        }
    }

    // Default alignment: none (left-aligned)
    let align = vec![AlignKind::None; col_count];

    Node::Table(Table {
        children: rows,
        align,
        position: None,
    })
}

/// Builds TableRow node from <tr>
pub(super) fn build_table_row(converter: &mut TypstHtmlConverter, handle: &Handle) -> Node {
    let mut cells = Vec::new();

    for child in handle.children.borrow().iter() {
        if let NodeData::Element { name, .. } = &child.data {
            let tag = name.local.as_ref();
            if tag == "th" || tag == "td" {
                cells.push(build_table_cell(converter, child));
            }
        }
    }

    Node::TableRow(TableRow {
        children: cells,
        position: None,
    })
}

/// Builds TableCell node from <th> or <td>
pub(super) fn build_table_cell(converter: &mut TypstHtmlConverter, handle: &Handle) -> Node {
    let saved_root = std::mem::take(&mut converter.root_children);
    let saved_para = converter.current_paragraph.take();

    // Process cell content
    for child in handle.children.borrow().iter() {
        converter.walk_node(child);
    }

    // Flush any remaining paragraph
    converter.end_paragraph();

    // Extract accumulated children
    let cell_children = std::mem::replace(&mut converter.root_children, saved_root);
    converter.current_paragraph = saved_para;

    // If no children, add empty text node
    let children = if cell_children.is_empty() {
        vec![Node::Text(Text {
            value: String::new(),
            position: None,
        })]
    } else {
        cell_children
    };

    Node::TableCell(TableCell {
        children,
        position: None,
    })
}
