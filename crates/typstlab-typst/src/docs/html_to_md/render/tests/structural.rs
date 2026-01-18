//! Structural rendering tests
//!
//! Tests for RenderResult and MdRender trait.

use crate::docs::html_to_md::render::{MdRender, RenderError, RenderResult};
use markdown::mdast::{AlignKind, Node};

/// Dummy renderer for testing trait
struct DummyRenderer;

impl MdRender for DummyRenderer {
    fn render(&self, node: &Node) -> Result<RenderResult, RenderError> {
        match node {
            Node::Text(text) => Ok(RenderResult::Inline(text.value.clone())),
            Node::Paragraph(_) => Ok(RenderResult::Block(vec!["[Paragraph]".to_string()])),
            Node::Table(_) => Ok(RenderResult::Table {
                rows: vec![vec!["A".to_string(), "B".to_string()]],
                align: vec![AlignKind::Left, AlignKind::Right],
            }),
            _ => Err(RenderError::UnsupportedNode(format!("{:?}", node))),
        }
    }
}

#[test]
fn test_render_result_inline() {
    let result = RenderResult::Inline("hello".to_string());

    assert!(matches!(result, RenderResult::Inline(_)));
    if let RenderResult::Inline(content) = result {
        assert_eq!(content, "hello");
    }
}

#[test]
fn test_render_result_block() {
    let result = RenderResult::Block(vec!["Line 1".to_string(), "Line 2".to_string()]);

    assert!(matches!(result, RenderResult::Block(_)));
    if let RenderResult::Block(lines) = result {
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Line 1");
        assert_eq!(lines[1], "Line 2");
    }
}

#[test]
fn test_render_result_table() {
    let result = RenderResult::Table {
        rows: vec![
            vec!["A".to_string(), "B".to_string()],
            vec!["1".to_string(), "2".to_string()],
        ],
        align: vec![AlignKind::Left, AlignKind::Right],
    };

    assert!(matches!(result, RenderResult::Table { .. }));
    if let RenderResult::Table { rows, align } = result {
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["A", "B"]);
        assert_eq!(rows[1], vec!["1", "2"]);
        assert_eq!(align.len(), 2);
        assert_eq!(align[0], AlignKind::Left);
        assert_eq!(align[1], AlignKind::Right);
    }
}

#[test]
fn test_render_result_equality() {
    let r1 = RenderResult::Inline("test".to_string());
    let r2 = RenderResult::Inline("test".to_string());
    let r3 = RenderResult::Inline("other".to_string());

    assert_eq!(r1, r2);
    assert_ne!(r1, r3);
}

#[test]
fn test_md_render_trait_inline() {
    let renderer = DummyRenderer;
    let node = Node::Text(markdown::mdast::Text {
        value: "hello".to_string(),
        position: None,
    });

    let result = renderer.render(&node).expect("Should render");

    assert!(matches!(result, RenderResult::Inline(_)));
    if let RenderResult::Inline(content) = result {
        assert_eq!(content, "hello");
    }
}

#[test]
fn test_md_render_trait_block() {
    let renderer = DummyRenderer;
    let node = Node::Paragraph(markdown::mdast::Paragraph {
        children: vec![],
        position: None,
    });

    let result = renderer.render(&node).expect("Should render");

    assert!(matches!(result, RenderResult::Block(_)));
}

#[test]
fn test_md_render_trait_table() {
    let renderer = DummyRenderer;
    let node = Node::Table(markdown::mdast::Table {
        children: vec![],
        align: vec![],
        position: None,
    });

    let result = renderer.render(&node).expect("Should render");

    assert!(matches!(result, RenderResult::Table { .. }));
}

#[test]
fn test_md_render_trait_unsupported() {
    let renderer = DummyRenderer;
    let node = Node::Code(markdown::mdast::Code {
        value: "code".to_string(),
        lang: None,
        meta: None,
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
fn test_render_error_display() {
    let err = RenderError::StandardFailed("test error".to_string());
    assert!(err.to_string().contains("Standard rendering failed"));
    assert!(err.to_string().contains("test error"));

    let err = RenderError::UnsupportedNode("Code".to_string());
    assert!(err.to_string().contains("Unsupported node type"));

    let err = RenderError::InvalidTable("empty rows".to_string());
    assert!(err.to_string().contains("Invalid table structure"));
}
