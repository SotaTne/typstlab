//! Function body rendering to Markdown
//!
//! Converts function definitions from docs.json to formatted Markdown.

use super::html_to_md;
use super::render::{default_value_to_text, extract_html_from_details, format_function_signature};
use super::schema::{FuncContent, ParamContent};
use thiserror::Error;

/// Renders function body to Markdown
///
/// Converts a function definition from docs.json into formatted Markdown with:
/// - Signature (name, params, return type)
/// - One-liner description
/// - Full details (HTML → Markdown)
/// - Example (if present)
/// - Parameters table (types, flags, defaults)
/// - Returns section
/// - Methods/scope (nested functions)
///
/// # Arguments
///
/// * `content` - JSON value containing function definition
///
/// # Returns
///
/// Formatted Markdown string
///
/// # Errors
///
/// Returns error if:
/// - JSON parsing fails
/// - HTML conversion fails
pub fn render_func_body(content: &serde_json::Value) -> Result<String, RenderError> {
    // Parse FuncContent from JSON
    let func: FuncContent = serde_json::from_value(content.clone())?;

    let mut md = String::new();

    // Function signature
    md.push_str("## Signature\n\n");
    md.push_str(&format_function_signature(&func));
    md.push_str("\n\n");

    // One-liner description
    if let Some(oneliner) = &func.oneliner {
        md.push_str(oneliner);
        md.push_str("\n\n");
    }

    // Full details (HTML → Markdown)
    if let Some(details) = &func.details {
        let details_html = extract_html_from_details(details);
        if !details_html.is_empty() {
            let details_md = html_to_md::convert(&details_html)?;
            md.push_str(&details_md);
            md.push_str("\n\n");
        }
    }

    // Example (if present)
    if let Some(example) = &func.example {
        md.push_str("## Example\n\n");
        let example_html = extract_html_from_details(example);
        if !example_html.is_empty() {
            let example_md = html_to_md::convert(&example_html)?;
            md.push_str(&example_md);
            md.push_str("\n\n");
        }
    }

    // Parameters
    if !func.params.is_empty() {
        md.push_str("## Parameters\n\n");
        for param in &func.params {
            md.push_str(&format_parameter(param)?);
        }
        md.push('\n');
    }

    // Returns
    if !func.returns.is_empty() {
        md.push_str("## Returns\n\n");
        md.push_str(&format!("`{}`\n\n", func.returns.join(" | ")));
    }

    // Scope (nested methods/properties)
    if !func.scope.is_empty() {
        md.push_str("## Methods\n\n");
        for method in &func.scope {
            md.push_str(&format_scoped_function(method)?);
            md.push('\n');
        }
    }

    Ok(md)
}

/// Formats single parameter with details
///
/// Creates a list item with:
/// - Parameter name (bold)
/// - Type(s) in parentheses
/// - Flags (required/optional, positional/named, variadic, settable)
/// - Default value (if present)
/// - Description (HTML → Markdown, indented)
/// - Example (if present, indented)
///
/// # Arguments
///
/// * `param` - Parameter content
///
/// # Returns
///
/// Formatted parameter Markdown
///
/// # Errors
///
/// Returns error if HTML conversion fails
fn format_parameter(param: &ParamContent) -> Result<String, RenderError> {
    let mut md = String::new();

    // Parameter name with type
    md.push_str(&format!("- **{}**", param.name));

    if !param.types.is_empty() {
        md.push_str(&format!(" (`{}`)", param.types.join(" | ")));
    }

    // Flags
    let mut flags = Vec::new();
    if param.required {
        flags.push("required");
    } else {
        flags.push("optional");
    }
    if param.positional {
        flags.push("positional");
    }
    if param.named {
        flags.push("named");
    }
    if param.variadic {
        flags.push("variadic");
    }
    if param.settable {
        flags.push("settable");
    }

    if !flags.is_empty() {
        md.push_str(&format!(", {}", flags.join(", ")));
    }

    // Default value
    if let Some(default) = &param.default {
        let default_str = default_value_to_text(default)?;
        md.push_str(&format!(", default: `{}`", default_str));
    }

    md.push_str(":\n");

    // Parameter description (HTML → Markdown, indented)
    if let Some(details) = &param.details {
        let details_html = extract_html_from_details(details);
        if !details_html.is_empty() {
            let details_md = html_to_md::convert(&details_html)?;
            for line in details_md.lines() {
                md.push_str("  ");
                md.push_str(line);
                md.push('\n');
            }
        }
    }

    // Parameter example (if present)
    if let Some(example) = &param.example {
        let example_html = extract_html_from_details(example);
        if !example_html.is_empty() {
            let example_md = html_to_md::convert(&example_html)?;
            md.push_str("  \n  Example:\n");
            for line in example_md.lines() {
                md.push_str("  ");
                md.push_str(line);
                md.push('\n');
            }
        }
    }

    md.push('\n');
    Ok(md)
}

/// Formats scoped function (method)
///
/// Renders a nested method/property with:
/// - h3 heading with method name
/// - Signature
/// - One-liner description
/// - Details (brief)
///
/// # Arguments
///
/// * `func` - Function content
///
/// # Returns
///
/// Formatted method Markdown
///
/// # Errors
///
/// Returns error if HTML conversion fails
fn format_scoped_function(func: &FuncContent) -> Result<String, RenderError> {
    let mut md = String::new();

    // Method heading
    md.push_str(&format!("### `{}`\n\n", func.name));

    // Signature
    md.push_str(&format!("{}\n\n", format_function_signature(func)));

    // One-liner
    if let Some(oneliner) = &func.oneliner {
        md.push_str(oneliner);
        md.push_str("\n\n");
    }

    // Details (brief)
    if let Some(details) = &func.details {
        let details_html = extract_html_from_details(details);
        if !details_html.is_empty() {
            let details_md = html_to_md::convert(&details_html)?;
            md.push_str(&details_md);
            md.push_str("\n\n");
        }
    }

    Ok(md)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_func_basic() {
        let fixture =
            include_str!("../../../../fixtures/typst/v0.12.0/test-fixtures/func-assert.json");
        let entry: crate::docs::schema::DocsEntry =
            serde_json::from_str(fixture).expect("Failed to parse func-assert.json");

        let body = entry.body.expect("Entry should have body");
        assert_eq!(body.kind, "func", "Body kind should be func");

        let result = render_func_body(&body.content).expect("Rendering should succeed");

        // Verify signature present
        assert!(
            result.contains("## Signature"),
            "Should have Signature section"
        );
        assert!(
            result.contains("`assert("),
            "Should have assert function signature"
        );

        // Verify parameters section
        assert!(
            result.contains("## Parameters"),
            "Should have Parameters section"
        );
        assert!(
            result.contains("**condition**"),
            "Should have condition parameter"
        );

        // Assert function may not have returns (returns nothing)
        // So we just verify the test ran successfully
    }

    #[test]
    fn test_render_func_with_scope() {
        let fixture =
            include_str!("../../../../fixtures/typst/v0.12.0/test-fixtures/type-array.json");
        let entry: crate::docs::schema::DocsEntry =
            serde_json::from_str(fixture).expect("Failed to parse type-array.json");

        let body = entry.body.expect("Entry should have body");
        assert_eq!(body.kind, "type", "Body kind should be type");

        // Extract a method from the scope
        let type_content: crate::docs::schema::TypeContent =
            serde_json::from_value(body.content.clone()).expect("Should parse TypeContent");

        assert!(!type_content.scope.is_empty(), "Type should have methods");

        // Test rendering one of the methods
        let method = &type_content.scope[0];
        let method_json = serde_json::to_value(method).expect("Should serialize method");
        let result = render_func_body(&method_json).expect("Rendering should succeed");

        // Verify method-specific sections
        assert!(
            result.contains("## Signature"),
            "Should have Signature section"
        );
        assert!(
            result.contains("## Parameters") || result.contains("## Returns"),
            "Should have Parameters or Returns section"
        );
    }

    /// Test: extract_html_from_details with array of Detail objects
    #[test]
    fn test_extract_html_from_details_array() {
        // Array with html and non-html items
        let details = serde_json::json!([
            {"kind": "html", "content": "<p>First paragraph</p>"},
            {"kind": "example", "content": "should be ignored"},
            {"kind": "html", "content": "<p>Second paragraph</p>"}
        ]);

        let result = extract_html_from_details(&details);

        assert_eq!(
            result, "<p>First paragraph</p>\n\n<p>Second paragraph</p>",
            "Should extract only html kind items and join with double newline"
        );
    }

    /// Test: extract_html_from_details with plain string
    #[test]
    fn test_extract_html_from_details_string() {
        let details = serde_json::json!("<p>Simple HTML</p>");

        let result = extract_html_from_details(&details);

        assert_eq!(result, "<p>Simple HTML</p>", "Should return string as-is");
    }

    /// Test: extract_html_from_details with empty/invalid input
    #[test]
    fn test_extract_html_from_details_empty() {
        let details = serde_json::json!(null);
        let result = extract_html_from_details(&details);
        assert_eq!(result, "", "Null should return empty string");

        let details = serde_json::json!([]);
        let result = extract_html_from_details(&details);
        assert_eq!(result, "", "Empty array should return empty string");

        let details = serde_json::json!(42);
        let result = extract_html_from_details(&details);
        assert_eq!(result, "", "Number should return empty string");
    }

    /// Test: Variadic parameter rendering
    #[test]
    fn test_render_variadic_parameter() {
        let func_json = serde_json::json!({
            "path": [],
            "name": "test_func",
            "title": "Test Function",
            "element": false,
            "contextual": false,
            "params": [{
                "name": "args",
                "types": ["any"],
                "required": false,
                "positional": true,
                "named": false,
                "variadic": true,
                "settable": false
            }],
            "returns": ["none"],
            "scope": []
        });

        let result = render_func_body(&func_json).expect("Should render variadic param");

        // Verify variadic flag is shown
        assert!(
            result.contains("variadic"),
            "Should show variadic flag for parameter"
        );
        assert!(result.contains("**args**"), "Should show parameter name");
    }

    /// Test: Default value rendering (array)
    #[test]
    fn test_render_default_array() {
        let func_json = serde_json::json!({
            "path": [],
            "name": "test_func",
            "title": "Test Function",
            "element": false,
            "contextual": false,
            "params": [{
                "name": "items",
                "types": ["array"],
                "default": [1, 2, 3],
                "required": false,
                "positional": false,
                "named": true,
                "variadic": false,
                "settable": false
            }],
            "returns": ["none"],
            "scope": []
        });

        let result = render_func_body(&func_json).expect("Should render array default");

        // Verify default value is shown as JSON
        assert!(result.contains("default:"), "Should show default label");
        assert!(
            result.contains("[1,2,3]") || result.contains("[1, 2, 3]"),
            "Should show array default value as JSON"
        );
    }

    /// Test: Default value rendering (object)
    #[test]
    fn test_render_default_object() {
        let func_json = serde_json::json!({
            "path": [],
            "name": "test_func",
            "title": "Test Function",
            "element": false,
            "contextual": false,
            "params": [{
                "name": "options",
                "types": ["dictionary"],
                "default": {"key": "value"},
                "required": false,
                "positional": false,
                "named": true,
                "variadic": false,
                "settable": false
            }],
            "returns": ["none"],
            "scope": []
        });

        let result = render_func_body(&func_json).expect("Should render object default");

        // Verify default value is shown as JSON
        assert!(result.contains("default:"), "Should show default label");
        assert!(
            result.contains("key") && result.contains("value"),
            "Should show object default value as JSON"
        );
    }

    /// Test HTML in default value is converted to plain text
    /// Real issue from docs.json where default values contain HTML markup
    #[test]
    fn test_default_value_html_stripped() {
        let func_json = serde_json::json!({
            "name": "cancel",
            "title": "Cancel",
            "category": "math",
            "oneliner": "Displays a diagonal line through content.",
            "element": true,
            "details": "<p>Displays a diagonal line through content.</p>",
            "params": [{
                "name": "length",
                "details": "<p>The length of the line.</p>",
                "example": null,
                "types": ["relative"],
                "strings": [],
                "default": "<code><span class=\"typ-num\">100%</span> <span class=\"typ-op\">+</span> <span class=\"typ-num\">3pt</span></code>",
                "positional": false,
                "named": true,
                "required": false,
                "variadic": false,
                "settable": true
            }],
            "returns": ["content"],
            "scope": []
        });

        let result = render_func_body(&func_json).expect("Should render HTML default");

        // Verify HTML tags are stripped from default value
        assert!(result.contains("default:"), "Should show default label");
        assert!(
            result.contains("100% + 3pt"),
            "Should extract plain text from HTML: found\n{}",
            result
        );
        assert!(
            !result.contains("<code>") && !result.contains("<span>"),
            "Should not contain HTML tags in output: found\n{}",
            result
        );
    }

    /// Test simple HTML in default value
    #[test]
    fn test_default_value_simple_html() {
        let func_json = serde_json::json!({
            "name": "test_func",
            "title": "Test",
            "category": "test",
            "oneliner": "Test function.",
            "element": true,
            "details": "<p>Test.</p>",
            "params": [{
                "name": "size",
                "details": "<p>The size.</p>",
                "example": null,
                "types": ["auto"],
                "strings": [],
                "default": "<code><span class=\"typ-key\">auto</span></code>",
                "positional": false,
                "named": true,
                "required": false,
                "variadic": false,
                "settable": true
            }],
            "returns": ["content"],
            "scope": []
        });

        let result = render_func_body(&func_json).expect("Should render simple HTML default");

        // Verify HTML is stripped to plain text
        assert!(
            result.contains("default: `auto`"),
            "Should show 'auto' without HTML tags: found\n{}",
            result
        );
        assert!(
            !result.contains("<code>") && !result.contains("<span>"),
            "Should not contain HTML tags: found\n{}",
            result
        );
    }
}

/// Function rendering errors
#[derive(Debug, Error)]
pub enum RenderError {
    /// JSON parsing error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// HTML conversion error
    #[error("HTML conversion error: {0}")]
    HtmlConversionError(#[from] html_to_md::ConversionError),
}
