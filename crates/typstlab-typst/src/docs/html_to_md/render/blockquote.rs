//! BlockquoteRenderer: Fast blockquote rendering
//!
//! Renders Blockquote nodes directly to Markdown without external libraries.
//! O(m) where m = nodes in blockquote.

use super::{MdRender, RenderError, RenderResult};
use markdown::mdast::{
    Blockquote, Emphasis, Heading, InlineCode, Link, Node, Paragraph, Strong, Text,
};

/// Blockquote renderer (fast, no external dependencies)
///
/// Renders blockquote content by traversing nodes and prefixing with >.
/// Much faster than mdast_util_to_markdown.
///
/// # Performance
///
/// O(m) where m = total nodes in blockquote
#[allow(dead_code)]
pub struct BlockquoteRenderer;

impl Default for BlockquoteRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockquoteRenderer {
    /// Create new BlockquoteRenderer
    #[allow(dead_code)]
    pub fn new() -> Self {
        BlockquoteRenderer
    }

    /// Render inline nodes to Markdown string
    fn render_inline_nodes(&self, nodes: &[Node]) -> String {
        nodes
            .iter()
            .map(|node| match node {
                Node::Text(Text { value, .. }) => value.clone(),
                Node::Emphasis(Emphasis { children, .. }) => {
                    format!("*{}*", self.render_inline_nodes(children))
                }
                Node::Strong(Strong { children, .. }) => {
                    format!("**{}**", self.render_inline_nodes(children))
                }
                Node::Link(Link { children, url, .. }) => {
                    format!("[{}]({})", self.render_inline_nodes(children), url)
                }
                Node::InlineCode(InlineCode { value, .. }) => {
                    format!("`{}`", value)
                }
                _ => String::new(),
            })
            .collect()
    }

    /// Render blockquote children to lines (without > prefix yet)
    fn render_blockquote_content(&self, children: &[Node]) -> Vec<String> {
        let mut lines = Vec::new();

        for child in children {
            match child {
                Node::Paragraph(Paragraph { children, .. }) => {
                    // Paragraph becomes single line
                    lines.push(self.render_inline_nodes(children));
                }
                Node::Heading(Heading {
                    depth, children, ..
                }) => {
                    // Heading with proper # symbols
                    let content = self.render_inline_nodes(children);
                    lines.push(format!("{} {}", "#".repeat(*depth as usize), content));
                }
                Node::Blockquote(nested) => {
                    // Nested blockquote - render recursively
                    let nested_lines = self.render_blockquote_content(&nested.children);
                    for line in nested_lines {
                        lines.push(format!("> {}", line));
                    }
                }
                _ => {
                    // Unknown node type - skip
                }
            }
        }

        lines
    }
}

impl MdRender for BlockquoteRenderer {
    fn render(&self, node: &Node) -> Result<RenderResult, RenderError> {
        let Node::Blockquote(Blockquote { children, .. }) = node else {
            return Err(RenderError::UnsupportedNode(format!(
                "BlockquoteRenderer only supports Blockquote nodes, got: {:?}",
                node
            )));
        };

        // Render content lines
        let content_lines = self.render_blockquote_content(children);

        // Prefix each line with >
        let quoted_lines: Vec<String> = content_lines
            .iter()
            .map(|line| format!("> {}", line))
            .collect();

        Ok(RenderResult::Block(quoted_lines))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blockquote_renderer_simple() {
        let renderer = BlockquoteRenderer::new();
        let blockquote = Node::Blockquote(Blockquote {
            children: vec![Node::Paragraph(Paragraph {
                children: vec![Node::Text(Text {
                    value: "Simple quote".to_string(),
                    position: None,
                })],
                position: None,
            })],
            position: None,
        });

        match renderer.render(&blockquote) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines.len(), 1);
                assert_eq!(lines[0], "> Simple quote");
            }
            _ => panic!("Expected Block result"),
        }
    }

    #[test]
    fn test_blockquote_renderer_multi_paragraph() {
        let renderer = BlockquoteRenderer::new();
        let blockquote = Node::Blockquote(Blockquote {
            children: vec![
                Node::Paragraph(Paragraph {
                    children: vec![Node::Text(Text {
                        value: "First paragraph".to_string(),
                        position: None,
                    })],
                    position: None,
                }),
                Node::Paragraph(Paragraph {
                    children: vec![Node::Text(Text {
                        value: "Second paragraph".to_string(),
                        position: None,
                    })],
                    position: None,
                }),
            ],
            position: None,
        });

        match renderer.render(&blockquote) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines.len(), 2);
                assert_eq!(lines[0], "> First paragraph");
                assert_eq!(lines[1], "> Second paragraph");
            }
            _ => panic!("Expected Block result"),
        }
    }

    #[test]
    fn test_blockquote_renderer_with_emphasis() {
        let renderer = BlockquoteRenderer::new();
        let blockquote = Node::Blockquote(Blockquote {
            children: vec![Node::Paragraph(Paragraph {
                children: vec![
                    Node::Text(Text {
                        value: "Quote with ".to_string(),
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
        });

        match renderer.render(&blockquote) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines[0], "> Quote with *emphasis*");
            }
            _ => panic!("Expected Block result"),
        }
    }

    #[test]
    fn test_blockquote_renderer_with_heading() {
        let renderer = BlockquoteRenderer::new();
        let blockquote = Node::Blockquote(Blockquote {
            children: vec![
                Node::Heading(Heading {
                    depth: 2,
                    children: vec![Node::Text(Text {
                        value: "Quoted Heading".to_string(),
                        position: None,
                    })],
                    position: None,
                }),
                Node::Paragraph(Paragraph {
                    children: vec![Node::Text(Text {
                        value: "Content".to_string(),
                        position: None,
                    })],
                    position: None,
                }),
            ],
            position: None,
        });

        match renderer.render(&blockquote) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines.len(), 2);
                assert_eq!(lines[0], "> ## Quoted Heading");
                assert_eq!(lines[1], "> Content");
            }
            _ => panic!("Expected Block result"),
        }
    }

    #[test]
    fn test_blockquote_renderer_nested() {
        let renderer = BlockquoteRenderer::new();
        let blockquote = Node::Blockquote(Blockquote {
            children: vec![
                Node::Paragraph(Paragraph {
                    children: vec![Node::Text(Text {
                        value: "Outer quote".to_string(),
                        position: None,
                    })],
                    position: None,
                }),
                Node::Blockquote(Blockquote {
                    children: vec![Node::Paragraph(Paragraph {
                        children: vec![Node::Text(Text {
                            value: "Inner quote".to_string(),
                            position: None,
                        })],
                        position: None,
                    })],
                    position: None,
                }),
            ],
            position: None,
        });

        match renderer.render(&blockquote) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines.len(), 2);
                assert_eq!(lines[0], "> Outer quote");
                assert_eq!(lines[1], "> > Inner quote");
            }
            _ => panic!("Expected Block result"),
        }
    }
}
