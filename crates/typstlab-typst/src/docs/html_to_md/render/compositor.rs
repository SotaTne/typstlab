//! Compositor: Assembles RenderResult into final Markdown
//!
//! Takes structural RenderResult and composes final Markdown strings.
//! Newlines derived from structure, not explicit in rendering logic.
//!
//! # Key Principle
//!
//! **NO explicit `\n` in composition logic** - only in final `join()` calls.
//! Structure determines formatting, not manual string insertion.

use super::RenderResult;
use markdown::mdast::AlignKind;

/// Compositor for assembling RenderResult into final Markdown
///
/// Composes structured results (Inline/Block/Table) into Markdown strings.
/// Newlines derived from block structure, never explicit in logic.
///
/// # Performance
///
/// O(n) composition where n = total content size.
#[allow(dead_code)] // Used in Phase 5+
pub struct Compositor;

impl Compositor {
    /// Create new Compositor
    #[allow(dead_code)] // Used in Phase 5+
    pub fn new() -> Self {
        Compositor
    }

    /// Compose structured results into final Markdown
    ///
    /// Newlines derived from block structure, not explicit.
    ///
    /// # Arguments
    ///
    /// * `results` - Structured render results to compose
    ///
    /// # Returns
    ///
    /// Final Markdown string with newlines from structure
    #[allow(dead_code)] // Used in Phase 5+
    pub fn compose(&self, results: Vec<RenderResult>) -> String {
        results
            .iter()
            .enumerate()
            .map(|(i, result)| match result {
                RenderResult::Inline(s) => s.clone(),
                RenderResult::Block(lines) => {
                    // Block: join lines, add spacing from context
                    let content = lines.join("\n");
                    if i < results.len() - 1 {
                        format!("{}\n\n", content) // Inter-block spacing
                    } else {
                        content
                    }
                }
                RenderResult::Table { rows, align } => {
                    // Format table from structure
                    let table = self.format_gfm_table(rows, align);
                    if i < results.len() - 1 {
                        format!("{}\n\n", table) // Inter-block spacing
                    } else {
                        table
                    }
                }
            })
            .collect::<Vec<_>>()
            .join("") // Join without separators (spacing already added)
    }

    /// Format GFM table from structure (newlines from join() only)
    ///
    /// # Arguments
    ///
    /// * `rows` - Table rows (pre-rendered cells)
    /// * `align` - Column alignments
    ///
    /// # Returns
    ///
    /// GFM-formatted table string
    #[allow(dead_code)] // Used in compose()
    fn format_gfm_table(&self, rows: &[Vec<String>], align: &[AlignKind]) -> String {
        if rows.is_empty() {
            return String::new();
        }

        // Calculate column widths from structure
        let col_widths = self.calculate_widths(rows);

        // Format rows (structure → lines)
        let formatted_rows: Vec<String> = rows
            .iter()
            .enumerate()
            .flat_map(|(i, row)| {
                let row_line = self.format_table_row(row, &col_widths);
                if i == 0 {
                    // Header: add separator after first row
                    vec![row_line, self.format_separator(align, &col_widths)]
                } else {
                    vec![row_line]
                }
            })
            .collect();

        // Join with newlines (derived from row structure)
        formatted_rows.join("\n")
    }

    /// Calculate column widths from row contents
    ///
    /// # Arguments
    ///
    /// * `rows` - Table rows
    ///
    /// # Returns
    ///
    /// Vector of column widths (minimum 3 for GFM)
    #[allow(dead_code)] // Used in format_gfm_table()
    fn calculate_widths(&self, rows: &[Vec<String>]) -> Vec<usize> {
        if rows.is_empty() {
            return vec![];
        }

        let col_count = rows[0].len();
        let mut widths = vec![0; col_count];

        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                widths[i] = widths[i].max(cell.len());
            }
        }

        // GFM minimum: 3 characters per column (for alignment markers)
        widths.iter().map(|&w| w.max(3)).collect()
    }

    /// Format a single table row
    ///
    /// # Arguments
    ///
    /// * `row` - Row cells
    /// * `col_widths` - Column widths
    ///
    /// # Returns
    ///
    /// Formatted row string (e.g., "| A   | B   |")
    #[allow(dead_code)] // Used in format_gfm_table()
    fn format_table_row(&self, row: &[String], col_widths: &[usize]) -> String {
        let cells: Vec<String> = row
            .iter()
            .zip(col_widths.iter())
            .map(|(cell, &width)| format!(" {:<width$} ", cell, width = width))
            .collect();

        format!("|{}|", cells.join("|"))
    }

    /// Format separator row with alignment
    ///
    /// # Arguments
    ///
    /// * `align` - Column alignments
    /// * `col_widths` - Column widths
    ///
    /// # Returns
    ///
    /// Separator string (e.g., "|:-----|-----:|")
    #[allow(dead_code)] // Used in format_gfm_table()
    fn format_separator(&self, align: &[AlignKind], col_widths: &[usize]) -> String {
        let separators: Vec<String> = align
            .iter()
            .zip(col_widths.iter())
            .map(|(align_kind, &width)| {
                let dashes = "-".repeat(width);
                match align_kind {
                    AlignKind::Left => format!(" :{} ", dashes),
                    AlignKind::Right => format!(" {}: ", dashes),
                    AlignKind::Center => format!(" :{}: ", dashes),
                    AlignKind::None => format!(" {} ", dashes),
                }
            })
            .collect();

        format!("|{}|", separators.join("|"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compositor_new() {
        let compositor = Compositor::new();
        assert!(matches!(compositor, Compositor));
    }

    #[test]
    fn test_compositor_inline_only() {
        let compositor = Compositor::new();
        let results = vec![RenderResult::Inline("Hello, world!".to_string())];

        let output = compositor.compose(results);
        assert_eq!(output, "Hello, world!");
    }

    #[test]
    fn test_compositor_blocks_with_spacing() {
        let compositor = Compositor::new();
        let results = vec![
            RenderResult::Block(vec!["# Heading".to_string()]),
            RenderResult::Block(vec!["Paragraph text".to_string()]),
        ];

        let output = compositor.compose(results);
        assert_eq!(output, "# Heading\n\nParagraph text");
    }

    #[test]
    fn test_compositor_mixed_content() {
        let compositor = Compositor::new();
        let results = vec![
            RenderResult::Block(vec!["# Title".to_string()]),
            RenderResult::Inline("inline".to_string()),
            RenderResult::Block(vec!["End".to_string()]),
        ];

        let output = compositor.compose(results);
        assert_eq!(output, "# Title\n\ninlineEnd");
    }

    #[test]
    fn test_compositor_calculate_widths() {
        let compositor = Compositor::new();
        let rows = vec![
            vec!["A".to_string(), "BB".to_string()],
            vec!["CCC".to_string(), "D".to_string()],
        ];

        let widths = compositor.calculate_widths(&rows);
        assert_eq!(widths, vec![3, 3]); // Minimum 3 for GFM
    }

    #[test]
    fn test_compositor_calculate_widths_large() {
        let compositor = Compositor::new();
        let rows = vec![
            vec!["Short".to_string(), "Longer Cell".to_string()],
            vec!["X".to_string(), "Y".to_string()],
        ];

        let widths = compositor.calculate_widths(&rows);
        assert_eq!(widths, vec![5, 11]);
    }

    #[test]
    fn test_compositor_format_table_row() {
        let compositor = Compositor::new();
        let row = vec!["A".to_string(), "B".to_string()];
        let widths = vec![3, 3];

        let output = compositor.format_table_row(&row, &widths);
        assert_eq!(output, "| A   | B   |");
    }

    #[test]
    fn test_compositor_format_separator_none() {
        let compositor = Compositor::new();
        let align = vec![AlignKind::None, AlignKind::None];
        let widths = vec![3, 3];

        let output = compositor.format_separator(&align, &widths);
        assert_eq!(output, "| --- | --- |");
    }

    #[test]
    fn test_compositor_format_separator_aligned() {
        let compositor = Compositor::new();
        let align = vec![AlignKind::Left, AlignKind::Right, AlignKind::Center];
        let widths = vec![5, 5, 5];

        let output = compositor.format_separator(&align, &widths);
        assert_eq!(output, "| :----- | -----: | :-----: |");
    }

    #[test]
    fn test_compositor_format_gfm_table_simple() {
        let compositor = Compositor::new();
        let rows = vec![
            vec!["Name".to_string(), "Age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
        ];
        let align = vec![AlignKind::None, AlignKind::None];

        let output = compositor.format_gfm_table(&rows, &align);
        let expected = "| Name  | Age |\n| ----- | --- |\n| Alice | 30  |";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_compositor_format_gfm_table_with_alignment() {
        let compositor = Compositor::new();
        let rows = vec![vec!["Left".to_string(), "Right".to_string()]];
        let align = vec![AlignKind::Left, AlignKind::Right];

        let output = compositor.format_gfm_table(&rows, &align);
        assert!(output.contains(":----"));
        assert!(output.contains("----:"));
    }

    #[test]
    fn test_compositor_table_in_context() {
        let compositor = Compositor::new();
        let results = vec![
            RenderResult::Block(vec!["# Table Example".to_string()]),
            RenderResult::Table {
                rows: vec![
                    vec!["A".to_string(), "B".to_string()],
                    vec!["1".to_string(), "2".to_string()],
                ],
                align: vec![AlignKind::None, AlignKind::None],
            },
            RenderResult::Block(vec!["End".to_string()]),
        ];

        let output = compositor.compose(results);
        assert!(output.starts_with("# Table Example\n\n"));
        assert!(output.contains("| A   | B   |"));
        assert!(output.contains("| 1   | 2   |"));
        assert!(output.ends_with("\n\nEnd"));
    }

    #[test]
    fn test_compositor_empty_table() {
        let compositor = Compositor::new();
        let rows: Vec<Vec<String>> = vec![];
        let align: Vec<AlignKind> = vec![];

        let output = compositor.format_gfm_table(&rows, &align);
        assert_eq!(output, "");
    }
}
