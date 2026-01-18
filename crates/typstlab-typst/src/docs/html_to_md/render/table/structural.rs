//! Structural GFM table renderer
//!
//! Renders mdast Table to GFM format structurally:
//! - Single-pass tree traversal (O(n))
//! - No explicit `\n` (structure determines formatting)
//! - Returns RenderResult::Table (not flat string)

use crate::docs::html_to_md::render::{MdRender, RenderError, RenderResult};
use markdown::mdast::{Node, TableCell, TableRow};

/// Structural GFM table renderer
///
/// Extracts table structure without manual string assembly.
/// Compositor handles final GFM formatting.
///
/// # Performance
///
/// O(n) where n = total nodes in table tree.
/// Step counter incremented on each render() call.
#[allow(dead_code)] // Used in Phase 5+
pub struct StructuralTableRenderer;

impl Default for StructuralTableRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl StructuralTableRenderer {
    /// Create new StructuralTableRenderer
    #[allow(dead_code)] // Used in Phase 5+
    pub fn new() -> Self {
        StructuralTableRenderer
    }

    /// Render cells in a row (O(k) for k cells)
    #[allow(dead_code)] // Used in render() and tests
    fn render_row_cells(&self, row: &TableRow) -> Vec<String> {
        row.children
            .iter()
            .filter_map(|cell| {
                if let Node::TableCell(cell_node) = cell {
                    Some(self.render_cell_inline(cell_node))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Render cell inline content (O(m) for m nodes)
    #[allow(dead_code)] // Used in render_row_cells() and tests
    fn render_cell_inline(&self, cell: &TableCell) -> String {
        cell.children
            .iter()
            .map(|child| self.render_inline_node(child))
            .collect::<Vec<_>>()
            .join("") // inline nodes: no separators
    }

    /// Render inline node to string (O(1))
    #[allow(dead_code)] // Used in render_cell_inline()
    fn render_inline_node(&self, node: &Node) -> String {
        // Record step for O(n) verification
        #[cfg(test)]
        {
            use super::super::tests::performance::test_counter;
            test_counter::inc();
        }

        match node {
            Node::Text(text) => Self::escape_pipe(&text.value),
            Node::InlineCode(code) => format!("`{}`", Self::escape_pipe(&code.value)),
            Node::Emphasis(em) => {
                let content = em
                    .children
                    .iter()
                    .map(|child| self.render_inline_node(child))
                    .collect::<Vec<_>>()
                    .join("");
                format!("*{}*", content)
            }
            Node::Strong(strong) => {
                let content = strong
                    .children
                    .iter()
                    .map(|child| self.render_inline_node(child))
                    .collect::<Vec<_>>()
                    .join("");
                format!("**{}**", content)
            }
            Node::Link(link) => {
                let text = link
                    .children
                    .iter()
                    .map(|child| self.render_inline_node(child))
                    .collect::<Vec<_>>()
                    .join("");
                format!("[{}]({})", text, link.url)
            }
            Node::Paragraph(para) => para
                .children
                .iter()
                .map(|child| self.render_inline_node(child))
                .collect::<Vec<_>>()
                .join(""),
            _ => String::new(),
        }
    }

    /// Escape pipe characters (`|` → `\|`)
    #[allow(dead_code)] // Used in render_inline_node()
    fn escape_pipe(text: &str) -> String {
        text.replace('|', "\\|")
    }
}

impl MdRender for StructuralTableRenderer {
    fn render(&self, node: &Node) -> Result<RenderResult, RenderError> {
        // Step counting done at node level in render_inline_node()
        let Node::Table(table) = node else {
            return Err(RenderError::UnsupportedNode("Not a Table node".to_string()));
        };

        // Extract structure (O(n) tree traversal)
        let rows: Vec<Vec<String>> = table
            .children
            .iter()
            .filter_map(|child| {
                if let Node::TableRow(row) = child {
                    Some(self.render_row_cells(row)) // O(k) cells
                } else {
                    None
                }
            })
            .collect();

        if rows.is_empty() {
            return Err(RenderError::InvalidTable("No rows found".to_string()));
        }

        // Return structural representation (NO explicit \n)
        Ok(RenderResult::Table {
            rows,
            align: table.align.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use markdown::mdast::{AlignKind, InlineCode, Strong, Table, Text};

    /// Helper: Create test table from string matrix
    fn create_test_table(cells: Vec<Vec<&str>>) -> markdown::mdast::Table {
        let rows = cells
            .iter()
            .map(|row| {
                let cell_nodes = row
                    .iter()
                    .map(|text| {
                        Node::TableCell(TableCell {
                            children: vec![Node::Text(Text {
                                value: text.to_string(),
                                position: None,
                            })],
                            position: None,
                        })
                    })
                    .collect();

                Node::TableRow(TableRow {
                    children: cell_nodes,
                    position: None,
                })
            })
            .collect();

        let col_count = cells.first().map(|r| r.len()).unwrap_or(0);

        Table {
            children: rows,
            align: vec![AlignKind::None; col_count],
            position: None,
        }
    }

    #[test]
    fn test_structural_table_renderer_new() {
        let renderer = StructuralTableRenderer::new();
        assert!(matches!(renderer, StructuralTableRenderer));
    }

    #[test]
    fn test_structural_table_simple() {
        let renderer = StructuralTableRenderer::new();
        let table = create_test_table(vec![vec!["A", "B"], vec!["1", "2"]]);

        let result = renderer.render(&Node::Table(table)).expect("Should render");

        assert!(matches!(result, RenderResult::Table { .. }));
        if let RenderResult::Table { rows, align } = result {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0], vec!["A", "B"]);
            assert_eq!(rows[1], vec!["1", "2"]);
            assert_eq!(align.len(), 2);
        }
    }

    #[test]
    fn test_structural_table_with_alignment() {
        let renderer = StructuralTableRenderer::new();
        let mut table = create_test_table(vec![vec!["Left", "Center", "Right"]]);
        table.align = vec![AlignKind::Left, AlignKind::Center, AlignKind::Right];

        let result = renderer
            .render(&Node::Table(table.clone()))
            .expect("Should render");

        if let RenderResult::Table { align, .. } = result {
            assert_eq!(align, table.align);
        }
    }

    #[test]
    fn test_structural_table_inline_formatting() {
        let renderer = StructuralTableRenderer::new();

        // Cell with inline code + strong
        let cell = TableCell {
            children: vec![
                Node::InlineCode(InlineCode {
                    value: "code".to_string(),
                    position: None,
                }),
                Node::Text(Text {
                    value: " and ".to_string(),
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
        };

        let result = renderer.render_cell_inline(&cell);
        assert_eq!(result, "`code` and **bold**");
    }

    #[test]
    fn test_structural_table_pipe_escaping() {
        let renderer = StructuralTableRenderer::new();
        let table = create_test_table(vec![vec!["A|B", "C|D|E"]]);

        let result = renderer.render(&Node::Table(table)).expect("Should render");

        if let RenderResult::Table { rows, .. } = result {
            assert_eq!(rows[0], vec!["A\\|B", "C\\|D\\|E"]);
        }
    }

    #[test]
    fn test_structural_table_empty_cells() {
        let renderer = StructuralTableRenderer::new();
        let table = create_test_table(vec![vec!["A", ""], vec!["", "B"]]);

        let result = renderer.render(&Node::Table(table)).expect("Should render");

        if let RenderResult::Table { rows, .. } = result {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0], vec!["A", ""]);
            assert_eq!(rows[1], vec!["", "B"]);
        }
    }

    #[test]
    fn test_structural_table_step_counter() {
        use super::super::super::tests::performance::test_counter;

        let renderer = StructuralTableRenderer::new();
        let table = create_test_table(vec![vec!["A"]]);

        test_counter::reset();
        assert_eq!(test_counter::get(), 0);

        let _ = renderer.render(&Node::Table(table));
        assert_eq!(test_counter::get(), 1);
    }

    #[test]
    fn test_structural_table_not_table_node() {
        let renderer = StructuralTableRenderer::new();
        let node = Node::Text(Text {
            value: "text".to_string(),
            position: None,
        });

        let result = renderer.render(&node);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RenderError::UnsupportedNode(_)
        ));
    }

    #[test]
    fn test_structural_table_empty_table() {
        let renderer = StructuralTableRenderer::new();
        let table = Table {
            children: vec![],
            align: vec![],
            position: None,
        };

        let result = renderer.render(&Node::Table(table));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RenderError::InvalidTable(_)));
    }
}
