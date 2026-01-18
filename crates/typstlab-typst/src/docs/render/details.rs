//! HTML details extraction from docs.json structures
//!
//! Handles extraction of HTML content from the details field, which can be:
//! - A simple string
//! - An array of Detail objects with kind="html"

/// Extract HTML content from details field (can be string or array of Detail objects)
///
/// The docs.json format supports two structures:
/// - String: `"<p>Some HTML</p>"`
/// - Array: `[{"kind": "html", "content": "<p>..."}, {"kind": "example", ...}]`
///
/// This function normalizes both formats to a single HTML string.
///
/// # Arguments
///
/// * `details` - JSON value containing details (string or array)
///
/// # Returns
///
/// HTML string (empty if no HTML content found)
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use typstlab_typst::docs::render::extract_html_from_details;
///
/// // String format
/// let details = json!("<p>Description</p>");
/// assert_eq!(extract_html_from_details(&details), "<p>Description</p>");
///
/// // Array format
/// let details = json!([
///     {"kind": "html", "content": "<p>First</p>"},
///     {"kind": "html", "content": "<p>Second</p>"}
/// ]);
/// assert_eq!(extract_html_from_details(&details), "<p>First</p>\n\n<p>Second</p>");
/// ```
pub fn extract_html_from_details(details: &serde_json::Value) -> String {
    match details {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|item| {
                let kind = item.get("kind")?.as_str()?;
                if kind == "html" {
                    item.get("content")?.as_str()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_string_format() {
        let details = json!("<p>Simple HTML string</p>");
        assert_eq!(
            extract_html_from_details(&details),
            "<p>Simple HTML string</p>"
        );
    }

    #[test]
    fn test_extract_array_format() {
        let details = json!([
            {"kind": "html", "content": "<p>First paragraph</p>"},
            {"kind": "html", "content": "<p>Second paragraph</p>"}
        ]);
        assert_eq!(
            extract_html_from_details(&details),
            "<p>First paragraph</p>\n\n<p>Second paragraph</p>"
        );
    }

    #[test]
    fn test_extract_array_with_mixed_kinds() {
        let details = json!([
            {"kind": "html", "content": "<p>HTML content</p>"},
            {"kind": "example", "content": "ignored"},
            {"kind": "html", "content": "<p>More HTML</p>"}
        ]);
        assert_eq!(
            extract_html_from_details(&details),
            "<p>HTML content</p>\n\n<p>More HTML</p>"
        );
    }

    #[test]
    fn test_extract_empty_array() {
        let details = json!([]);
        assert_eq!(extract_html_from_details(&details), "");
    }

    #[test]
    fn test_extract_null() {
        let details = json!(null);
        assert_eq!(extract_html_from_details(&details), "");
    }

    #[test]
    fn test_extract_invalid_structure() {
        let details = json!({"not": "expected"});
        assert_eq!(extract_html_from_details(&details), "");
    }
}
