//! Structural GFM table renderer
//!
//! Renders mdast Table nodes to GFM format by extracting structure,
//! never using explicit `\n`. Newlines derived from row structure.

mod structural;

pub use structural::StructuralTableRenderer;
