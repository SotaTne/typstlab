//! ParagraphRenderer: Fast paragraph rendering
//!
//! Renders Paragraph nodes directly to Markdown without external libraries.
//! O(m) where m = inline nodes in paragraph.

use super::{MdRender, RenderError, RenderResult};
use markdown::mdast::{Emphasis, InlineCode, Link, Node, Paragraph, Strong, Text};

/// Paragraph renderer (fast, no external dependencies)
///
/// Renders paragraph content by traversing inline nodes.
/// Much faster than mdast_util_to_markdown.
///
/// # Performance
///
/// O(m) where m = number of inline nodes
#[allow(dead_code)]
pub struct ParagraphRenderer;

impl Default for ParagraphRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl ParagraphRenderer {
    /// Create new ParagraphRenderer
    #[allow(dead_code)]
    pub fn new() -> Self {
        ParagraphRenderer
    }

    /// Render inline nodes to Markdown string
    ///
    /// Handles: Text, Emphasis, Strong, Link, InlineCode
    ///
    /// # Arguments
    ///
    /// * `nodes` - Inline nodes to render
    ///
    /// # Returns
    ///
    /// Markdown string (no trailing newlines)
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
                _ => String::new(), // Unknown inline node
            })
            .collect()
    }
}

impl MdRender for ParagraphRenderer {
    fn render(&self, node: &Node) -> Result<RenderResult, RenderError> {
        let Node::Paragraph(Paragraph { children, .. }) = node else {
            return Err(RenderError::UnsupportedNode(format!(
                "ParagraphRenderer only supports Paragraph nodes, got: {:?}",
                node
            )));
        };

        // Render inline content
        let content = self.render_inline_nodes(children);

        // Return as single-line block (Compositor will add spacing)
        Ok(RenderResult::Block(vec![content]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paragraph_renderer_simple() {
        let renderer = ParagraphRenderer::new();
        let para = Node::Paragraph(Paragraph {
            children: vec![Node::Text(Text {
                value: "Simple text".to_string(),
                position: None,
            })],
            position: None,
        });

        match renderer.render(&para) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines.len(), 1);
                assert_eq!(lines[0], "Simple text");
            }
            _ => panic!("Expected Block result"),
        }
    }

    #[test]
    fn test_paragraph_renderer_with_emphasis() {
        let renderer = ParagraphRenderer::new();
        let para = Node::Paragraph(Paragraph {
            children: vec![
                Node::Text(Text {
                    value: "Text with ".to_string(),
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
        });

        match renderer.render(&para) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines[0], "Text with *emphasis*");
            }
            _ => panic!("Expected Block result"),
        }
    }

    #[test]
    fn test_paragraph_renderer_with_strong() {
        let renderer = ParagraphRenderer::new();
        let para = Node::Paragraph(Paragraph {
            children: vec![
                Node::Text(Text {
                    value: "Text with ".to_string(),
                    position: None,
                }),
                Node::Strong(Strong {
                    children: vec![Node::Text(Text {
                        value: "bold".to_string(),
                        position: None,
                    })],
                    position: None,
                }),
            ],
            position: None,
        });

        match renderer.render(&para) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines[0], "Text with **bold**");
            }
            _ => panic!("Expected Block result"),
        }
    }

    #[test]
    fn test_paragraph_renderer_with_link() {
        let renderer = ParagraphRenderer::new();
        let para = Node::Paragraph(Paragraph {
            children: vec![Node::Link(Link {
                children: vec![Node::Text(Text {
                    value: "link text".to_string(),
                    position: None,
                })],
                url: "https://example.com".to_string(),
                title: None,
                position: None,
            })],
            position: None,
        });

        match renderer.render(&para) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines[0], "[link text](https://example.com)");
            }
            _ => panic!("Expected Block result"),
        }
    }

    #[test]
    fn test_paragraph_renderer_with_inline_code() {
        let renderer = ParagraphRenderer::new();
        let para = Node::Paragraph(Paragraph {
            children: vec![
                Node::Text(Text {
                    value: "Inline ".to_string(),
                    position: None,
                }),
                Node::InlineCode(InlineCode {
                    value: "code()".to_string(),
                    position: None,
                }),
            ],
            position: None,
        });

        match renderer.render(&para) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines[0], "Inline `code()`");
            }
            _ => panic!("Expected Block result"),
        }
    }
}
