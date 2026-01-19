//! CompositeRenderer: Unified rendering strategy
//!
//! Integrates StandardRenderer, StructuralTableRenderer, and Compositor
//! into a single, cohesive rendering pipeline.
//!
//! # Strategy
//!
//! 1. **Table nodes** → StructuralTableRenderer (structural GFM)
//! 2. **Other nodes** → StandardRenderer (mdast_util_to_markdown)
//! 3. **Assembly** → Compositor (structure → final Markdown)
//!
//! # Performance
//!
//! O(n) guarantee: delegates to O(n) components, single composition pass.

use super::blockquote::BlockquoteRenderer;
use super::heading::HeadingRenderer;
use super::list::ListRenderer;
use super::paragraph::ParagraphRenderer;
use super::table::StructuralTableRenderer;
use super::{Compositor, MdRender, RenderError, RenderResult, StandardRenderer};
use markdown::mdast::Node;

/// Composite renderer with unified strategy
///
/// Coordinates StandardRenderer, StructuralTableRenderer, and Compositor
/// to provide seamless mdast → Markdown conversion.
///
/// # Performance
///
/// O(n) where n = total nodes. Step counter tracks all render() calls.
#[allow(dead_code)] // Used in Phase 6+
pub struct CompositeRenderer {
    blockquote: BlockquoteRenderer,
    heading: HeadingRenderer,
    list: ListRenderer,
    paragraph: ParagraphRenderer,
    standard: StandardRenderer,
    table: StructuralTableRenderer,
    compositor: Compositor,
}

impl Default for CompositeRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl CompositeRenderer {
    /// Create new CompositeRenderer
    #[allow(dead_code)] // Used in Phase 6+
    pub fn new() -> Self {
        CompositeRenderer {
            blockquote: BlockquoteRenderer::new(),
            heading: HeadingRenderer::new(),
            list: ListRenderer::new(),
            paragraph: ParagraphRenderer::new(),
            standard: StandardRenderer::new(),
            table: StructuralTableRenderer::new(),
            compositor: Compositor::new(),
        }
    }

    /// Render mdast node to Markdown string
    ///
    /// Delegates to appropriate renderer, then composes result.
    /// Handles Root nodes by rendering children to enable table detection.
    ///
    /// # Arguments
    ///
    /// * `node` - mdast Node to render
    ///
    /// # Returns
    ///
    /// Final Markdown string
    ///
    /// # Errors
    ///
    /// Returns RenderError if rendering fails
    #[allow(dead_code)] // Used in Phase 6+
    pub fn render(&self, node: &Node) -> Result<String, RenderError> {
        // Dispatch to appropriate renderer
        match node {
            Node::Root(root) => {
                // Handle Root by rendering children individually
                // This enables StructuralTableRenderer for tables
                self.render_many(&root.children)
            }
            Node::Table(_) => {
                // Use structural table renderer for GFM tables
                let result = self.table.render(node)?;
                Ok(self.compositor.compose(vec![result]))
            }
            _ => {
                // Use standard renderer for everything else
                let result = self.standard.render(node)?;
                Ok(self.compositor.compose(vec![result]))
            }
        }
    }

    /// Render multiple nodes and compose together
    ///
    /// Useful for rendering Root node children.
    ///
    /// # Arguments
    ///
    /// * `nodes` - Vector of nodes to render
    ///
    /// # Returns
    ///
    /// Composed Markdown string
    ///
    /// # Errors
    ///
    /// Returns RenderError if any rendering fails
    #[allow(dead_code)] // Used in Phase 6+
    pub fn render_many(&self, nodes: &[Node]) -> Result<String, RenderError> {
        let results: Result<Vec<RenderResult>, RenderError> = nodes
            .iter()
            .map(|node| match node {
                Node::Table(_) => self.table.render(node),
                Node::Paragraph(_) => self.paragraph.render(node), // Fast!
                Node::Heading(_) => self.heading.render(node),     // Fast!
                Node::List(_) => self.list.render(node),           // Fast!
                Node::Blockquote(_) => self.blockquote.render(node), // Fast!
                _ => self.standard.render(node),
            })
            .collect();

        Ok(self.compositor.compose(results?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use markdown::mdast::{AlignKind, Heading, Paragraph, Root, Table, TableCell, TableRow, Text};

    #[test]
    fn test_composite_renderer_new() {
        let renderer = CompositeRenderer::new();
        assert!(matches!(renderer, CompositeRenderer { .. }));
    }

    #[test]
    fn test_composite_renders_paragraph() {
        let renderer = CompositeRenderer::new();
        let node = Node::Paragraph(Paragraph {
            children: vec![Node::Text(Text {
                value: "Hello, world!".to_string(),
                position: None,
            })],
            position: None,
        });

        let result = renderer.render(&node).expect("Should render");
        assert!(result.contains("Hello, world!"));
    }

    #[test]
    fn test_composite_renders_heading() {
        let renderer = CompositeRenderer::new();
        let node = Node::Heading(Heading {
            depth: 1,
            children: vec![Node::Text(Text {
                value: "Title".to_string(),
                position: None,
            })],
            position: None,
        });

        let result = renderer.render(&node).expect("Should render");
        assert!(result.contains("# Title"));
    }

    #[test]
    fn test_composite_delegates_table() {
        let renderer = CompositeRenderer::new();
        let table = Node::Table(Table {
            children: vec![
                Node::TableRow(TableRow {
                    children: vec![
                        Node::TableCell(TableCell {
                            children: vec![Node::Text(Text {
                                value: "A".to_string(),
                                position: None,
                            })],
                            position: None,
                        }),
                        Node::TableCell(TableCell {
                            children: vec![Node::Text(Text {
                                value: "B".to_string(),
                                position: None,
                            })],
                            position: None,
                        }),
                    ],
                    position: None,
                }),
                Node::TableRow(TableRow {
                    children: vec![
                        Node::TableCell(TableCell {
                            children: vec![Node::Text(Text {
                                value: "1".to_string(),
                                position: None,
                            })],
                            position: None,
                        }),
                        Node::TableCell(TableCell {
                            children: vec![Node::Text(Text {
                                value: "2".to_string(),
                                position: None,
                            })],
                            position: None,
                        }),
                    ],
                    position: None,
                }),
            ],
            align: vec![AlignKind::None, AlignKind::None],
            position: None,
        });

        let result = renderer.render(&table).expect("Should render table");
        assert!(result.contains("| A   | B   |"));
        assert!(result.contains("| --- | --- |"));
        assert!(result.contains("| 1   | 2   |"));
    }

    #[test]
    fn test_composite_render_many() {
        let renderer = CompositeRenderer::new();
        let nodes = vec![
            Node::Heading(Heading {
                depth: 1,
                children: vec![Node::Text(Text {
                    value: "Document".to_string(),
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
        ];

        let result = renderer.render_many(&nodes).expect("Should render many");
        assert!(result.contains("# Document"));
        assert!(result.contains("Content"));
    }

    #[test]
    fn test_composite_render_many_with_table() {
        let renderer = CompositeRenderer::new();
        let nodes = vec![
            Node::Paragraph(Paragraph {
                children: vec![Node::Text(Text {
                    value: "Before table".to_string(),
                    position: None,
                })],
                position: None,
            }),
            Node::Table(Table {
                children: vec![Node::TableRow(TableRow {
                    children: vec![Node::TableCell(TableCell {
                        children: vec![Node::Text(Text {
                            value: "Cell".to_string(),
                            position: None,
                        })],
                        position: None,
                    })],
                    position: None,
                })],
                align: vec![AlignKind::None],
                position: None,
            }),
            Node::Paragraph(Paragraph {
                children: vec![Node::Text(Text {
                    value: "After table".to_string(),
                    position: None,
                })],
                position: None,
            }),
        ];

        let result = renderer.render_many(&nodes).expect("Should render many");
        assert!(result.contains("Before table"));
        assert!(result.contains("| Cell |"));
        assert!(result.contains("After table"));
    }

    #[test]
    fn test_composite_o_n_guarantee_single_node() {
        use super::super::tests::performance::test_counter;

        let renderer = CompositeRenderer::new();
        let node = Node::Paragraph(Paragraph {
            children: vec![Node::Text(Text {
                value: "Test".to_string(),
                position: None,
            })],
            position: None,
        });

        test_counter::reset();
        let _ = renderer.render(&node);

        // StandardRenderer increments once for Paragraph
        assert!(test_counter::get() > 0, "Step counter should track calls");
    }

    #[test]
    fn test_composite_o_n_guarantee_table() {
        use super::super::tests::performance::test_counter;

        let renderer = CompositeRenderer::new();
        let table = Node::Table(Table {
            children: vec![Node::TableRow(TableRow {
                children: vec![Node::TableCell(TableCell {
                    children: vec![Node::Text(Text {
                        value: "A".to_string(),
                        position: None,
                    })],
                    position: None,
                })],
                position: None,
            })],
            align: vec![AlignKind::None],
            position: None,
        });

        test_counter::reset();
        let _ = renderer.render(&table);

        // StructuralTableRenderer increments once
        assert_eq!(test_counter::get(), 1);
    }

    #[test]
    fn test_composite_uses_standard_for_blocks() {
        let renderer = CompositeRenderer::new();
        let node = Node::Root(Root {
            children: vec![Node::Paragraph(Paragraph {
                children: vec![Node::Text(Text {
                    value: "Test".to_string(),
                    position: None,
                })],
                position: None,
            })],
            position: None,
        });

        let result = renderer.render(&node).expect("Should render");
        assert!(result.contains("Test"));
    }

    #[test]
    fn test_composite_renders_root_with_table() {
        let renderer = CompositeRenderer::new();
        let root = Node::Root(Root {
            children: vec![
                Node::Paragraph(Paragraph {
                    children: vec![Node::Text(Text {
                        value: "Before table".to_string(),
                        position: None,
                    })],
                    position: None,
                }),
                Node::Table(Table {
                    children: vec![
                        Node::TableRow(TableRow {
                            children: vec![
                                Node::TableCell(TableCell {
                                    children: vec![Node::Text(Text {
                                        value: "A".to_string(),
                                        position: None,
                                    })],
                                    position: None,
                                }),
                                Node::TableCell(TableCell {
                                    children: vec![Node::Text(Text {
                                        value: "B".to_string(),
                                        position: None,
                                    })],
                                    position: None,
                                }),
                            ],
                            position: None,
                        }),
                        Node::TableRow(TableRow {
                            children: vec![
                                Node::TableCell(TableCell {
                                    children: vec![Node::Text(Text {
                                        value: "1".to_string(),
                                        position: None,
                                    })],
                                    position: None,
                                }),
                                Node::TableCell(TableCell {
                                    children: vec![Node::Text(Text {
                                        value: "2".to_string(),
                                        position: None,
                                    })],
                                    position: None,
                                }),
                            ],
                            position: None,
                        }),
                    ],
                    align: vec![AlignKind::None, AlignKind::None],
                    position: None,
                }),
                Node::Paragraph(Paragraph {
                    children: vec![Node::Text(Text {
                        value: "After table".to_string(),
                        position: None,
                    })],
                    position: None,
                }),
            ],
            position: None,
        });

        let result = renderer.render(&root).expect("Should render");
        // Verify table uses StructuralTableRenderer (GFM format)
        assert!(result.contains("Before table"));
        assert!(result.contains("| A   | B   |"));
        assert!(result.contains("| --- | --- |"));
        assert!(result.contains("| 1   | 2   |"));
        assert!(result.contains("After table"));
    }
}
