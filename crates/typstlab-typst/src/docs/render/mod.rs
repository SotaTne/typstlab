//! Markdown rendering utilities for docs.json structures
//!
//! This module provides shared rendering functions used across different body types:
//! - HTML details extraction
//! - Function signature formatting
//! - Default value text extraction

mod details;
mod signature;

use html5ever::{parse_fragment, tendril::TendrilSink};
use markup5ever::namespace_url;
use markup5ever_rcdom::{Handle, NodeData, RcDom};

// Re-export public APIs
pub use details::extract_html_from_details;
pub use signature::format_function_signature;

/// Converts a default value from docs.json to plain text.
///
/// If the value is a string containing HTML markup (common in docs.json default values),
/// parses the HTML and extracts only the text content. This prevents HTML tags from
/// appearing in the rendered Markdown output.
///
/// For non-string values (arrays, objects, etc.), falls back to JSON serialization.
///
/// # Arguments
///
/// * `value` - The default value from docs.json
///
/// # Returns
///
/// Plain text representation of the default value
///
/// # Errors
///
/// Returns error if HTML parsing fails or JSON serialization fails
pub fn default_value_to_text(
    value: &serde_json::Value,
) -> Result<String, crate::docs::render_func::RenderError> {
    match value {
        serde_json::Value::String(s) => {
            // If it looks like HTML, parse and extract text
            if s.contains('<') && s.contains('>') {
                extract_text_from_html(s)
            } else {
                // Plain string, return as-is
                Ok(s.clone())
            }
        }
        _ => {
            // For non-string values (arrays, objects, numbers, etc.),
            // use JSON serialization
            serde_json::to_string(value).map_err(crate::docs::render_func::RenderError::JsonError)
        }
    }
}

/// Extracts plain text from HTML string
///
/// Parses the HTML and recursively collects all text nodes,
/// ignoring all HTML tags and attributes.
fn extract_text_from_html(html: &str) -> Result<String, crate::docs::render_func::RenderError> {
    let dom = parse_fragment(
        RcDom::default(),
        Default::default(),
        markup5ever::QualName::new(
            None,
            markup5ever::ns!(html),
            markup5ever::local_name!("div"),
        ),
        vec![],
    )
    .from_utf8()
    .read_from(&mut html.as_bytes());

    match dom {
        Ok(dom) => Ok(collect_text_from_handle(&dom.document)),
        Err(_) => {
            // If parsing fails, return original string
            // (better than failing completely)
            Ok(html.to_string())
        }
    }
}

/// Recursively collects text content from an HTML node tree
fn collect_text_from_handle(handle: &Handle) -> String {
    let mut text = String::new();
    for child in handle.children.borrow().iter() {
        match &child.data {
            NodeData::Text { contents } => {
                text.push_str(&contents.borrow());
            }
            NodeData::Element { .. } => {
                text.push_str(&collect_text_from_handle(child));
            }
            _ => {}
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_value_plain_string() {
        let value = serde_json::json!("plain text");
        let result = default_value_to_text(&value).unwrap();
        assert_eq!(result, "plain text");
    }

    #[test]
    fn test_default_value_html_simple() {
        let value = serde_json::json!("<code>100%</code>");
        let result = default_value_to_text(&value).unwrap();
        assert_eq!(result, "100%");
    }

    #[test]
    fn test_default_value_html_nested_spans() {
        // Real example from docs.json
        let value = serde_json::json!(
            "<code><span class=\"typ-num\">100%</span> <span class=\"typ-op\">+</span> <span class=\"typ-num\">0pt</span></code>"
        );
        let result = default_value_to_text(&value).unwrap();
        assert_eq!(result, "100% + 0pt");
    }

    #[test]
    fn test_default_value_html_comparison_operators() {
        // HTML entities are only decoded during HTML parsing.
        // If the string contains raw entities without HTML tags,
        // it's treated as plain text.
        let value = serde_json::json!("&lt;code&gt;auto&lt;/code&gt;");
        let result = default_value_to_text(&value).unwrap();
        // Since there are no < or > characters, it's treated as plain text
        assert_eq!(result, "&lt;code&gt;auto&lt;/code&gt;");
    }

    #[test]
    fn test_default_value_number() {
        let value = serde_json::json!(42);
        let result = default_value_to_text(&value).unwrap();
        assert_eq!(result, "42");
    }

    #[test]
    fn test_default_value_boolean() {
        let value = serde_json::json!(true);
        let result = default_value_to_text(&value).unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn test_default_value_array() {
        let value = serde_json::json!(["a", "b", "c"]);
        let result = default_value_to_text(&value).unwrap();
        assert_eq!(result, "[\"a\",\"b\",\"c\"]");
    }

    #[test]
    fn test_default_value_object() {
        let value = serde_json::json!({"key": "value"});
        let result = default_value_to_text(&value).unwrap();
        assert_eq!(result, "{\"key\":\"value\"}");
    }
}
