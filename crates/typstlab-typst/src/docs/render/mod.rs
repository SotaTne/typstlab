//! Markdown rendering utilities for docs.json structures
//!
//! This module provides shared rendering functions used across different body types:
//! - HTML details extraction
//! - Function signature formatting

mod details;
mod signature;

// Re-export public APIs
pub use details::extract_html_from_details;
pub use signature::format_function_signature;
