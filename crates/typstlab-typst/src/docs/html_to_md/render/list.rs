//! ListRenderer: Fast list rendering
//!
//! Renders List nodes directly to Markdown without external libraries.
//! O(m) where m = total nodes in list (including nested lists).

use super::{MdRender, RenderError, RenderResult};
use markdown::mdast::{Emphasis, InlineCode, Link, List, ListItem, Node, Paragraph, Strong, Text};

/// List renderer (fast, no external dependencies)
///
/// Renders ordered and unordered lists with proper nesting.
/// Much faster than mdast_util_to_markdown.
///
/// # Performance
///
/// O(m) where m = total nodes in list tree
#[allow(dead_code)]
pub struct ListRenderer;

impl Default for ListRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl ListRenderer {
    /// Create new ListRenderer
    #[allow(dead_code)]
    pub fn new() -> Self {
        ListRenderer
    }

    /// Render inline nodes to Markdown string
    fn render_inline_nodes(nodes: &[Node]) -> String {
        nodes
            .iter()
            .map(|node| match node {
                Node::Text(Text { value, .. }) => value.clone(),
                Node::Emphasis(Emphasis { children, .. }) => {
                    format!("*{}*", Self::render_inline_nodes(children))
                }
                Node::Strong(Strong { children, .. }) => {
                    format!("**{}**", Self::render_inline_nodes(children))
                }
                Node::Link(Link { children, url, .. }) => {
                    format!("[{}]({})", Self::render_inline_nodes(children), url)
                }
                Node::InlineCode(InlineCode { value, .. }) => {
                    format!("`{}`", value)
                }
                _ => String::new(),
            })
            .collect()
    }

    /// Render a list item's content
    fn render_list_item_content(&self, children: &[Node]) -> Vec<String> {
        let mut lines = Vec::new();

        for child in children {
            match child {
                Node::Paragraph(Paragraph { children, .. }) => {
                    // Inline content becomes single line
                    lines.push(Self::render_inline_nodes(children));
                }
                Node::List(nested_list) => {
                    // Nested list - render with increased indentation
                    let nested_lines = self.render_list_lines(nested_list, 1);
                    lines.extend(nested_lines);
                }
                _ => {
                    // Unknown node type - skip
                }
            }
        }

        lines
    }

    /// Render list to lines with given indent level
    fn render_list_lines(&self, list: &List, indent_level: usize) -> Vec<String> {
        let indent = "  ".repeat(indent_level);
        let mut lines = Vec::new();

        for (index, child) in list.children.iter().enumerate() {
            if let Node::ListItem(ListItem { children, .. }) = child {
                let item_lines = self.render_list_item_content(children);

                if !item_lines.is_empty() {
                    // First line gets the bullet/number
                    let prefix = if list.ordered {
                        format!("{}{}. ", indent, list.start.unwrap_or(1) + index as u32)
                    } else {
                        format!("{}- ", indent)
                    };

                    lines.push(format!("{}{}", prefix, item_lines[0]));

                    // Subsequent lines are indented
                    for line in &item_lines[1..] {
                        if line.starts_with("  ") {
                            // Already indented (nested list)
                            lines.push(format!("{}{}", indent, line));
                        } else {
                            // Regular continuation line
                            lines.push(format!("{}  {}", indent, line));
                        }
                    }
                }
            }
        }

        lines
    }
}

impl MdRender for ListRenderer {
    fn render(&self, node: &Node) -> Result<RenderResult, RenderError> {
        let Node::List(list) = node else {
            return Err(RenderError::UnsupportedNode(format!(
                "ListRenderer only supports List nodes, got: {:?}",
                node
            )));
        };

        // Render list with base indentation level 0
        let lines = self.render_list_lines(list, 0);

        Ok(RenderResult::Block(lines))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_renderer_unordered_simple() {
        let renderer = ListRenderer::new();
        let list = Node::List(List {
            ordered: false,
            start: None,
            spread: false,
            children: vec![
                Node::ListItem(ListItem {
                    checked: None,
                    spread: false,
                    children: vec![Node::Paragraph(Paragraph {
                        children: vec![Node::Text(Text {
                            value: "Item 1".to_string(),
                            position: None,
                        })],
                        position: None,
                    })],
                    position: None,
                }),
                Node::ListItem(ListItem {
                    checked: None,
                    spread: false,
                    children: vec![Node::Paragraph(Paragraph {
                        children: vec![Node::Text(Text {
                            value: "Item 2".to_string(),
                            position: None,
                        })],
                        position: None,
                    })],
                    position: None,
                }),
            ],
            position: None,
        });

        match renderer.render(&list) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines.len(), 2);
                assert_eq!(lines[0], "- Item 1");
                assert_eq!(lines[1], "- Item 2");
            }
            _ => panic!("Expected Block result"),
        }
    }

    #[test]
    fn test_list_renderer_ordered() {
        let renderer = ListRenderer::new();
        let list = Node::List(List {
            ordered: true,
            start: Some(1),
            spread: false,
            children: vec![
                Node::ListItem(ListItem {
                    checked: None,
                    spread: false,
                    children: vec![Node::Paragraph(Paragraph {
                        children: vec![Node::Text(Text {
                            value: "First".to_string(),
                            position: None,
                        })],
                        position: None,
                    })],
                    position: None,
                }),
                Node::ListItem(ListItem {
                    checked: None,
                    spread: false,
                    children: vec![Node::Paragraph(Paragraph {
                        children: vec![Node::Text(Text {
                            value: "Second".to_string(),
                            position: None,
                        })],
                        position: None,
                    })],
                    position: None,
                }),
            ],
            position: None,
        });

        match renderer.render(&list) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines[0], "1. First");
                assert_eq!(lines[1], "2. Second");
            }
            _ => panic!("Expected Block result"),
        }
    }

    #[test]
    fn test_list_renderer_with_emphasis() {
        let renderer = ListRenderer::new();
        let list = Node::List(List {
            ordered: false,
            start: None,
            spread: false,
            children: vec![Node::ListItem(ListItem {
                checked: None,
                spread: false,
                children: vec![Node::Paragraph(Paragraph {
                    children: vec![
                        Node::Text(Text {
                            value: "Item with ".to_string(),
                            position: None,
                        }),
                        Node::Emphasis(Emphasis {
                            children: vec![Node::Text(Text {
                                value: "emphasis".to_string(),
                                position: None,
                            })],
                            position: None,
                        }),
                    ],
                    position: None,
                })],
                position: None,
            })],
            position: None,
        });

        match renderer.render(&list) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines[0], "- Item with *emphasis*");
            }
            _ => panic!("Expected Block result"),
        }
    }

    #[test]
    fn test_list_renderer_nested() {
        let renderer = ListRenderer::new();
        let list = Node::List(List {
            ordered: false,
            start: None,
            spread: false,
            children: vec![
                Node::ListItem(ListItem {
                    checked: None,
                    spread: false,
                    children: vec![
                        Node::Paragraph(Paragraph {
                            children: vec![Node::Text(Text {
                                value: "Parent 1".to_string(),
                                position: None,
                            })],
                            position: None,
                        }),
                        Node::List(List {
                            ordered: false,
                            start: None,
                            spread: false,
                            children: vec![Node::ListItem(ListItem {
                                checked: None,
                                spread: false,
                                children: vec![Node::Paragraph(Paragraph {
                                    children: vec![Node::Text(Text {
                                        value: "Child 1".to_string(),
                                        position: None,
                                    })],
                                    position: None,
                                })],
                                position: None,
                            })],
                            position: None,
                        }),
                    ],
                    position: None,
                }),
                Node::ListItem(ListItem {
                    checked: None,
                    spread: false,
                    children: vec![Node::Paragraph(Paragraph {
                        children: vec![Node::Text(Text {
                            value: "Parent 2".to_string(),
                            position: None,
                        })],
                        position: None,
                    })],
                    position: None,
                }),
            ],
            position: None,
        });

        match renderer.render(&list) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines.len(), 3);
                assert_eq!(lines[0], "- Parent 1");
                assert_eq!(lines[1], "  - Child 1");
                assert_eq!(lines[2], "- Parent 2");
            }
            _ => panic!("Expected Block result"),
        }
    }
}
