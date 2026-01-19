//! HeadingRenderer: Fast heading rendering
//!
//! Renders Heading nodes directly to Markdown without external libraries.
//! O(m) where m = inline nodes in heading.

use super::{MdRender, RenderError, RenderResult};
use markdown::mdast::{Emphasis, Heading, InlineCode, Link, Node, Strong, Text};

/// Heading renderer (fast, no external dependencies)
///
/// Renders heading content by traversing inline nodes.
/// Much faster than mdast_util_to_markdown.
///
/// # Performance
///
/// O(m) where m = number of inline nodes
#[allow(dead_code)]
pub struct HeadingRenderer;

impl Default for HeadingRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadingRenderer {
    /// Create new HeadingRenderer
    #[allow(dead_code)]
    pub fn new() -> Self {
        HeadingRenderer
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

impl MdRender for HeadingRenderer {
    fn render(&self, node: &Node) -> Result<RenderResult, RenderError> {
        let Node::Heading(Heading {
            depth, children, ..
        }) = node
        else {
            return Err(RenderError::UnsupportedNode(format!(
                "HeadingRenderer only supports Heading nodes, got: {:?}",
                node
            )));
        };

        // Render inline content
        let content = self.render_inline_nodes(children);

        // Format with appropriate number of # symbols
        let heading = format!("{} {}", "#".repeat(*depth as usize), content);

        // Return as single-line block (Compositor will add spacing)
        Ok(RenderResult::Block(vec![heading]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_renderer_h1() {
        let renderer = HeadingRenderer::new();
        let heading = Node::Heading(Heading {
            depth: 1,
            children: vec![Node::Text(Text {
                value: "Title".to_string(),
                position: None,
            })],
            position: None,
        });

        match renderer.render(&heading) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines.len(), 1);
                assert_eq!(lines[0], "# Title");
            }
            _ => panic!("Expected Block result"),
        }
    }

    #[test]
    fn test_heading_renderer_h2() {
        let renderer = HeadingRenderer::new();
        let heading = Node::Heading(Heading {
            depth: 2,
            children: vec![Node::Text(Text {
                value: "Subtitle".to_string(),
                position: None,
            })],
            position: None,
        });

        match renderer.render(&heading) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines[0], "## Subtitle");
            }
            _ => panic!("Expected Block result"),
        }
    }

    #[test]
    fn test_heading_renderer_h6() {
        let renderer = HeadingRenderer::new();
        let heading = Node::Heading(Heading {
            depth: 6,
            children: vec![Node::Text(Text {
                value: "Deep heading".to_string(),
                position: None,
            })],
            position: None,
        });

        match renderer.render(&heading) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines[0], "###### Deep heading");
            }
            _ => panic!("Expected Block result"),
        }
    }

    #[test]
    fn test_heading_renderer_with_emphasis() {
        let renderer = HeadingRenderer::new();
        let heading = Node::Heading(Heading {
            depth: 1,
            children: vec![
                Node::Text(Text {
                    value: "Title with ".to_string(),
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

        match renderer.render(&heading) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines[0], "# Title with *emphasis*");
            }
            _ => panic!("Expected Block result"),
        }
    }

    #[test]
    fn test_heading_renderer_with_link() {
        let renderer = HeadingRenderer::new();
        let heading = Node::Heading(Heading {
            depth: 2,
            children: vec![
                Node::Text(Text {
                    value: "Check ".to_string(),
                    position: None,
                }),
                Node::Link(Link {
                    children: vec![Node::Text(Text {
                        value: "this link".to_string(),
                        position: None,
                    })],
                    url: "https://example.com".to_string(),
                    title: None,
                    position: None,
                }),
            ],
            position: None,
        });

        match renderer.render(&heading) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines[0], "## Check [this link](https://example.com)");
            }
            _ => panic!("Expected Block result"),
        }
    }

    #[test]
    fn test_heading_renderer_with_inline_code() {
        let renderer = HeadingRenderer::new();
        let heading = Node::Heading(Heading {
            depth: 3,
            children: vec![
                Node::Text(Text {
                    value: "Using ".to_string(),
                    position: None,
                }),
                Node::InlineCode(InlineCode {
                    value: "function()".to_string(),
                    position: None,
                }),
            ],
            position: None,
        });

        match renderer.render(&heading) {
            Ok(RenderResult::Block(lines)) => {
                assert_eq!(lines[0], "### Using `function()`");
            }
            _ => panic!("Expected Block result"),
        }
    }
}
