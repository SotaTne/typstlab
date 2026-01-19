//! Structural mdast rendering
//!
//! Provides trait-based rendering of mdast nodes to Markdown strings.
//! Renders by traversing mdast tree structure, never using explicit `\n`.
//! Whitespace and newlines are derived from structure, not manual insertion.
//!
//! # Architecture
//!
//! - **MdRender trait**: Structural rendering abstraction
//! - **RenderResult**: Preserves structure until composition
//! - **StandardRenderer**: Thin wrapper over `mdast_util_to_markdown`
//! - **StructuralTableRenderer**: GFM table rendering from structure
//! - **Compositor**: Assembles RenderResult into final Markdown
//! - **CompositeRenderer**: Unified rendering strategy
//!
//! # Performance Guarantee
//!
//! All renderers guarantee O(n) complexity where n = node count.
//! Verified with step counters in `tests/performance.rs`.

mod composite;
mod compositor;
mod heading;
mod paragraph;
mod standard;
mod table;

#[allow(unused_imports)] // Used in Phase 6+
pub use composite::CompositeRenderer;
#[allow(unused_imports)] // Used in Phase 5+
pub use compositor::Compositor;
#[allow(unused_imports)] // Used in Phase 10+
pub use heading::HeadingRenderer;
#[allow(unused_imports)] // Used in Phase 10+
pub use paragraph::ParagraphRenderer;
#[allow(unused_imports)] // Used in Phase 5+
pub use standard::StandardRenderer;
#[allow(unused_imports)] // Used in Phase 5+
pub use table::StructuralTableRenderer;

use markdown::mdast::{AlignKind, Node};
use thiserror::Error;

#[cfg(test)]
mod tests;

/// Rendering output (structural, not flat string)
#[allow(dead_code)] // Used in Phase 2+
///
/// Preserves structure until final composition.
/// Rendering logic extracts structure; Compositor derives newlines from structure.
///
/// # Structural Rendering Principle
///
/// Renderers (StandardRenderer, StructuralTableRenderer) do NOT insert explicit `\n`.
/// They return RenderResult with structure.
/// Compositor assembles RenderResult and inserts newlines based on context.
#[derive(Debug, Clone, PartialEq)]
pub enum RenderResult {
    /// Inline content (no newlines)
    ///
    /// Example: "**bold** text" from Strong + Text nodes
    Inline(String),

    /// Block content (newlines before/after derived from context)
    ///
    /// Each string is a block line. Compositor adds inter-block spacing.
    /// Example: vec!["# Heading", "Paragraph text"]
    Block(Vec<String>),

    /// Structured table (rows as vec, formatting from structure)
    ///
    /// Compositor formats GFM table from row/alignment structure.
    /// Example: rows=vec![vec!["A", "B"], vec!["1", "2"]], align=[Left, Right]
    Table {
        /// Cell content by row (pre-rendered as strings)
        rows: Vec<Vec<String>>,
        /// Column alignment
        align: Vec<AlignKind>,
    },
}

/// Trait for structural mdast rendering
#[allow(dead_code)] // Used in Phase 2+
///
/// Renders mdast nodes by traversing structure, not manual string ops.
/// Whitespace/newlines derived from structure, never explicit.
///
/// # Performance
///
/// All implementations must guarantee O(n) where n = node count.
/// Use step counters (`test_counter::inc()`) in test builds.
pub trait MdRender {
    /// Render node structurally (O(1) per node)
    ///
    /// Returns rendered fragments; structure determines composition.
    ///
    /// # Arguments
    ///
    /// * `node` - mdast Node to render
    ///
    /// # Returns
    ///
    /// Structural result (Inline/Block/Table), not flat string.
    ///
    /// # Errors
    ///
    /// Returns RenderError if rendering fails (e.g., unsupported node type).
    fn render(&self, node: &Node) -> Result<RenderResult, RenderError>;
}

/// Rendering errors
#[allow(dead_code)] // Used in Phase 2+
#[derive(Debug, Error)]
pub enum RenderError {
    /// Standard renderer (mdast_util_to_markdown) failed
    #[error("Standard rendering failed: {0}")]
    StandardFailed(String),

    /// Node type not supported by this renderer
    #[error("Unsupported node type: {0}")]
    UnsupportedNode(String),

    /// Table structure invalid
    #[error("Invalid table structure: {0}")]
    InvalidTable(String),
}

/// Create a CompositeRenderer with default configuration
///
/// Factory function for creating a unified renderer that coordinates
/// StandardRenderer, StructuralTableRenderer, and Compositor.
///
/// # Returns
///
/// CompositeRenderer instance ready for use
///
/// # Example
///
/// ```
/// use typstlab_typst::docs::html_to_md::render::create_composite_renderer;
/// use markdown::mdast::{Node, Text};
///
/// let renderer = create_composite_renderer();
/// // let result = renderer.render(&node);
/// ```
#[allow(dead_code)] // Used in Phase 6+
pub fn create_composite_renderer() -> CompositeRenderer {
    CompositeRenderer::new()
}
