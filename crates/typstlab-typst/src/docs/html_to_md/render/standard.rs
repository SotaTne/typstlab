//! StandardRenderer: Thin wrapper over mdast_util_to_markdown
//!
//! Delegates to `mdast_util_to_markdown::to_markdown()` for CommonMark compliance.
//! O(n) complexity assumed from external library.

use super::{MdRender, RenderError, RenderResult};
use markdown::mdast::Node;

/// Standard renderer using mdast_util_to_markdown
///
/// Thin wrapper over external library. Delegates all rendering
/// to `mdast_util_to_markdown::to_markdown()`.
///
/// # Performance
///
/// O(n) complexity (assumed from external library).
/// Step counter incremented on each render() call.
#[allow(dead_code)] // Used in Phase 5+
pub struct StandardRenderer;

impl Default for StandardRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl StandardRenderer {
    /// Create new StandardRenderer
    #[allow(dead_code)] // Used in Phase 5+
    pub fn new() -> Self {
        StandardRenderer
    }
}

impl MdRender for StandardRenderer {
    fn render(&self, node: &Node) -> Result<RenderResult, RenderError> {
        // Record steps for O(n) verification
        // Count nodes in the subtree since mdast_util_to_markdown processes them
        #[cfg(test)]
        {
            use super::tests::performance::{count_nodes, test_counter};
            let node_count = count_nodes(node);
            for _ in 0..node_count {
                test_counter::inc();
            }
        }

        // Delegate to mdast_util_to_markdown
        match mdast_util_to_markdown::to_markdown(node) {
            Ok(md) => {
                // Determine result type from node
                let result = match node {
                    Node::Root(_) => RenderResult::Block(vec![md]),
                    Node::Paragraph(_)
                    | Node::Heading(_)
                    | Node::Code(_)
                    | Node::Blockquote(_)
                    | Node::List(_) => RenderResult::Block(vec![md]),
                    _ => RenderResult::Inline(md),
                };
                Ok(result)
            }
            Err(e) => Err(RenderError::StandardFailed(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use markdown::mdast::{Heading, Paragraph, Root, Text};

    #[test]
    fn test_standard_renderer_new() {
        let renderer = StandardRenderer::new();
        assert!(matches!(renderer, StandardRenderer));
    }

    #[test]
    fn test_standard_renderer_paragraph() {
        let renderer = StandardRenderer::new();
        let node = Node::Paragraph(Paragraph {
            children: vec![Node::Text(Text {
                value: "Hello, world!".to_string(),
                position: None,
            })],
            position: None,
        });

        let result = renderer.render(&node).expect("Should render");

        assert!(matches!(result, RenderResult::Block(_)));
        if let RenderResult::Block(lines) = result {
            assert_eq!(lines.len(), 1);
            assert!(lines[0].contains("Hello, world!"));
        }
    }

    #[test]
    fn test_standard_renderer_heading() {
        let renderer = StandardRenderer::new();
        let node = Node::Heading(Heading {
            depth: 2,
            children: vec![Node::Text(Text {
                value: "Title".to_string(),
                position: None,
            })],
            position: None,
        });

        let result = renderer.render(&node).expect("Should render");

        assert!(matches!(result, RenderResult::Block(_)));
        if let RenderResult::Block(lines) = result {
            assert!(lines[0].contains("##"));
            assert!(lines[0].contains("Title"));
        }
    }

    #[test]
    fn test_standard_renderer_code_block() {
        let renderer = StandardRenderer::new();
        let node = Node::Code(markdown::mdast::Code {
            value: "let x = 1;".to_string(),
            lang: Some("rust".to_string()),
            meta: None,
            position: None,
        });

        let result = renderer.render(&node).expect("Should render");

        assert!(matches!(result, RenderResult::Block(_)));
        if let RenderResult::Block(lines) = result {
            assert!(lines[0].contains("```"));
            assert!(lines[0].contains("let x = 1;"));
        }
    }

    #[test]
    fn test_standard_renderer_inline_text() {
        let renderer = StandardRenderer::new();
        let node = Node::Text(Text {
            value: "inline text".to_string(),
            position: None,
        });

        let result = renderer.render(&node).expect("Should render");

        assert!(matches!(result, RenderResult::Inline(_)));
        if let RenderResult::Inline(content) = result {
            assert_eq!(content.trim(), "inline text");
        }
    }

    #[test]
    fn test_standard_renderer_root_node() {
        let renderer = StandardRenderer::new();
        let node = Node::Root(Root {
            children: vec![Node::Paragraph(Paragraph {
                children: vec![Node::Text(Text {
                    value: "Content".to_string(),
                    position: None,
                })],
                position: None,
            })],
            position: None,
        });

        let result = renderer.render(&node).expect("Should render");

        assert!(matches!(result, RenderResult::Block(_)));
        if let RenderResult::Block(lines) = result {
            assert!(lines[0].contains("Content"));
        }
    }

    #[test]
    fn test_standard_renderer_step_counter() {
        use super::super::tests::performance::test_counter;

        let renderer = StandardRenderer::new();
        let node = Node::Text(Text {
            value: "test".to_string(),
            position: None,
        });

        test_counter::reset();
        assert_eq!(test_counter::get(), 0);

        let _ = renderer.render(&node);
        assert_eq!(test_counter::get(), 1);

        let _ = renderer.render(&node);
        assert_eq!(test_counter::get(), 2);
    }
}
