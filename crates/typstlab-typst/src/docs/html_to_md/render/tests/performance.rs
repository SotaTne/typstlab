//! O(n) Performance Verification Tests for mdast Rendering
//!
//! Verifies that mdast rendering is O(n) where n = node count.
//! Uses step counters (like template/engine) to verify algorithmic complexity.

use markdown::mdast::{AlignKind, Node, Paragraph, Root, Table, TableCell, TableRow, Text};

/// Thread-local step counter for O(n) verification
///
/// Pattern from crates/typstlab-core/src/template/engine/tokenize.rs
#[cfg(test)]
pub(crate) mod test_counter {
    use std::cell::Cell;

    thread_local! {
        static RENDER_STEPS: Cell<usize> = const { Cell::new(0) };
    }

    pub(crate) fn reset() {
        RENDER_STEPS.with(|c| c.set(0));
    }

    pub(crate) fn inc() {
        RENDER_STEPS.with(|c| c.set(c.get() + 1));
    }

    pub(crate) fn get() -> usize {
        RENDER_STEPS.with(|c| c.get())
    }
}

/// Helper: Count total nodes in mdast tree
fn count_nodes(node: &Node) -> usize {
    1 + match node {
        Node::Root(root) => root.children.iter().map(count_nodes).sum(),
        Node::Paragraph(para) => para.children.iter().map(count_nodes).sum(),
        Node::Table(table) => table.children.iter().map(count_nodes).sum(),
        Node::TableRow(row) => row.children.iter().map(count_nodes).sum(),
        Node::TableCell(cell) => cell.children.iter().map(count_nodes).sum(),
        _ => 0,
    }
}

/// Helper: Create test tree with N nodes
fn create_test_tree(node_count: usize) -> Node {
    // Create paragraphs with text nodes
    // Each paragraph = 2 nodes (Paragraph + Text)
    let para_count = node_count / 2;
    let children = (0..para_count)
        .map(|i| {
            Node::Paragraph(Paragraph {
                children: vec![Node::Text(Text {
                    value: format!("Paragraph {}", i),
                    position: None,
                })],
                position: None,
            })
        })
        .collect();

    Node::Root(Root {
        children,
        position: None,
    })
}

/// Helper: Create deeply nested tree (depth D)
fn create_deeply_nested_tree(depth: usize) -> Node {
    fn create_nested(current_depth: usize, max_depth: usize) -> Node {
        if current_depth >= max_depth {
            Node::Text(Text {
                value: "Leaf".to_string(),
                position: None,
            })
        } else {
            Node::Paragraph(Paragraph {
                children: vec![create_nested(current_depth + 1, max_depth)],
                position: None,
            })
        }
    }

    Node::Root(Root {
        children: vec![create_nested(0, depth)],
        position: None,
    })
}

/// Helper: Create wide table (R rows × C columns)
fn create_wide_table(rows: usize, cols: usize) -> Node {
    let table_rows = (0..rows)
        .map(|r| {
            let cells = (0..cols)
                .map(|c| {
                    Node::TableCell(TableCell {
                        children: vec![Node::Text(Text {
                            value: format!("Cell_{r}_{c}"),
                            position: None,
                        })],
                        position: None,
                    })
                })
                .collect();

            Node::TableRow(TableRow {
                children: cells,
                position: None,
            })
        })
        .collect();

    Node::Table(Table {
        children: table_rows,
        align: vec![AlignKind::None; cols],
        position: None,
    })
}

/// Helper: Run function with step counter
fn with_step_counter<F: FnOnce()>(f: F) -> usize {
    test_counter::reset();
    f();
    test_counter::get()
}

/// Calculate steps-per-node ratio
fn steps_per_node(steps: usize, nodes: usize) -> f64 {
    steps as f64 / nodes as f64
}

#[test]
#[ignore] // Placeholder: will pass when renderers are implemented
fn test_render_o_n_performance() {
    // Generate mdast trees of different sizes
    let tree_100 = create_test_tree(100); // ~100 nodes
    let tree_1000 = create_test_tree(1000); // ~1000 nodes
    let tree_10000 = create_test_tree(10000); // ~10000 nodes

    // Count actual nodes
    let nodes_100 = count_nodes(&tree_100);
    let nodes_1000 = count_nodes(&tree_1000);
    let nodes_10000 = count_nodes(&tree_10000);

    println!(
        "Tree sizes: {} nodes, {} nodes, {} nodes",
        nodes_100, nodes_1000, nodes_10000
    );

    // Count steps for each size (placeholder - will fail until renderer implemented)
    let steps_100 = with_step_counter(|| {
        // TODO: Implement in Phase 5
        // let renderer = create_composite_renderer();
        // renderer.render(&tree_100);
        eprintln!("Renderer not yet implemented (Phase 0 placeholder)");
    });

    let steps_1000 = with_step_counter(|| {
        // TODO: Implement in Phase 5
        eprintln!("Renderer not yet implemented (Phase 0 placeholder)");
    });

    let steps_10000 = with_step_counter(|| {
        // TODO: Implement in Phase 5
        eprintln!("Renderer not yet implemented (Phase 0 placeholder)");
    });

    // Verify O(n) scaling: steps-per-node should be constant
    let spn_100 = steps_per_node(steps_100, nodes_100);
    let spn_1000 = steps_per_node(steps_1000, nodes_1000);
    let spn_10000 = steps_per_node(steps_10000, nodes_10000);

    println!(
        "Steps-per-node: {:.3}, {:.3}, {:.3}",
        spn_100, spn_1000, spn_10000
    );

    let avg_spn = (spn_100 + spn_1000 + spn_10000) / 3.0;
    let tolerance = avg_spn * 0.2; // 20% variance

    assert!(
        (spn_100 - avg_spn).abs() <= tolerance,
        "Steps-per-node variance too high: {:.3} vs avg {:.3}",
        spn_100,
        avg_spn
    );
    assert!(
        (spn_1000 - avg_spn).abs() <= tolerance,
        "Steps-per-node variance too high: {:.3} vs avg {:.3}",
        spn_1000,
        avg_spn
    );
    assert!(
        (spn_10000 - avg_spn).abs() <= tolerance,
        "Steps-per-node variance too high: {:.3} vs avg {:.3}",
        spn_10000,
        avg_spn
    );

    // Absolute upper bound: steps <= 3.0 * node_count
    assert!(
        steps_100 <= nodes_100 * 3,
        "Steps {} exceeded 3x node count {}",
        steps_100,
        nodes_100
    );
    assert!(
        steps_1000 <= nodes_1000 * 3,
        "Steps {} exceeded 3x node count {}",
        steps_1000,
        nodes_1000
    );
    assert!(
        steps_10000 <= nodes_10000 * 3,
        "Steps {} exceeded 3x node count {}",
        steps_10000,
        nodes_10000
    );
}

#[test]
#[ignore] // Placeholder: will pass when renderers are implemented
fn test_render_worst_case_deep_nesting() {
    // Worst case: deeply nested structure (depth 100)
    let tree = create_deeply_nested_tree(100);
    let node_count = count_nodes(&tree);

    println!("Deep nesting tree: {} nodes (depth 100)", node_count);

    let steps = with_step_counter(|| {
        // TODO: Implement in Phase 5
        eprintln!("Renderer not yet implemented (Phase 0 placeholder)");
    });

    println!("Steps: {}, Node count: {}", steps, node_count);

    // Should still be O(n) despite depth
    assert!(
        steps <= node_count * 3,
        "Steps {} exceeded 3x node count {}",
        steps,
        node_count
    );
}

#[test]
#[ignore] // Placeholder: will pass when renderers are implemented
fn test_render_worst_case_wide_tables() {
    // Worst case: wide tables (100 rows × 50 columns = 5000 cells)
    let tree = create_wide_table(100, 50);
    let node_count = count_nodes(&tree);

    println!("Wide table tree: {} nodes (100×50)", node_count);

    let steps = with_step_counter(|| {
        // TODO: Implement in Phase 5
        eprintln!("Renderer not yet implemented (Phase 0 placeholder)");
    });

    println!("Steps: {}, Node count: {}", steps, node_count);

    // Should still be O(n) despite width
    assert!(
        steps <= node_count * 3,
        "Steps {} exceeded 3x node count {}",
        steps,
        node_count
    );
}

#[test]
fn test_count_nodes_simple_tree() {
    // Verify count_nodes helper
    let tree = Node::Root(Root {
        children: vec![Node::Paragraph(Paragraph {
            children: vec![Node::Text(Text {
                value: "Hello".to_string(),
                position: None,
            })],
            position: None,
        })],
        position: None,
    });

    // Root (1) + Paragraph (1) + Text (1) = 3
    assert_eq!(count_nodes(&tree), 3);
}

#[test]
fn test_count_nodes_table() {
    // Verify count_nodes for tables
    let table = create_wide_table(2, 3); // 2 rows × 3 columns

    // Table (1) + TableRow (2) + TableCell (6) + Text (6) = 15
    assert_eq!(count_nodes(&table), 15);
}

#[test]
fn test_step_counter_isolation() {
    // Verify step counter works in isolation
    test_counter::reset();
    assert_eq!(test_counter::get(), 0);

    test_counter::inc();
    assert_eq!(test_counter::get(), 1);

    test_counter::inc();
    test_counter::inc();
    assert_eq!(test_counter::get(), 3);

    test_counter::reset();
    assert_eq!(test_counter::get(), 0);
}
