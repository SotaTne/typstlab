//! Body type renderers for non-function content
//!
//! Provides specialized renderers for:
//! - Type definitions (with constructor and methods)
//! - Category listings
//! - Function groups
//! - Symbol tables

use super::html_to_md;
use super::render::{extract_html_from_details, format_function_signature};
use super::render_func;
use super::schema::{CategoryContent, FuncContent, GroupContent, SymbolsContent, TypeContent};
use thiserror::Error;

/// Renders type body to Markdown
///
/// Converts a type definition from docs.json into formatted Markdown with:
/// - Type description
/// - Constructor (if present)
/// - Methods/properties
///
/// # Arguments
///
/// * `content` - JSON value containing type definition
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
pub fn render_type_body(content: &serde_json::Value) -> Result<String, RenderError> {
    let type_content: TypeContent = serde_json::from_value(content.clone())?;

    let mut md = String::new();

    // Type description
    if let Some(details) = &type_content.details {
        let details_html = extract_html_from_details(details);
        if !details_html.is_empty() {
            md.push_str(&html_to_md::convert(&details_html)?);
            md.push_str("\n\n");
        }
    }

    // Constructor (if present)
    if let Some(constructor) = &type_content.constructor {
        md.push_str("## Constructor\n\n");
        let constructor_json = serde_json::to_value(constructor)?;
        md.push_str(&render_func::render_func_body(&constructor_json)?);
    }

    // Methods/properties
    if !type_content.scope.is_empty() {
        md.push_str("## Methods\n\n");
        for method in &type_content.scope {
            md.push_str(&format_scoped_function(method)?);
        }
    }

    Ok(md)
}

/// Renders category body to Markdown
///
/// Converts a category listing from docs.json into formatted Markdown with:
/// - Category description
/// - Items listing (linked)
///
/// # Arguments
///
/// * `content` - JSON value containing category definition
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
pub fn render_category_body(content: &serde_json::Value) -> Result<String, RenderError> {
    let cat: CategoryContent = serde_json::from_value(content.clone())?;

    let mut md = String::new();

    // Category description
    if let Some(details) = &cat.details {
        let details_html = extract_html_from_details(details);
        if !details_html.is_empty() {
            md.push_str(&html_to_md::convert(&details_html)?);
            md.push_str("\n\n");
        }
    }

    // Items listing
    if !cat.items.is_empty() {
        md.push_str("## Items\n\n");
        for item in &cat.items {
            // Rewrite internal links using smart URL parser
            let fixed_route = crate::docs::links::rewrite_docs_link(&item.route).into_owned();
            md.push_str(&format!("- [{}]({})", item.name, fixed_route));
            if let Some(oneliner) = &item.oneliner {
                md.push_str(&format!(" - {}", oneliner));
            }
            md.push('\n');
        }
        md.push('\n');
    }

    Ok(md)
}

/// Renders group body to Markdown
///
/// Converts a function group from docs.json into formatted Markdown with:
/// - Group description
/// - Function listings with oneliners
///
/// # Arguments
///
/// * `content` - JSON value containing group definition
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
pub fn render_group_body(content: &serde_json::Value) -> Result<String, RenderError> {
    let group: GroupContent = serde_json::from_value(content.clone())?;

    let mut md = String::new();

    // Group description
    if let Some(details) = &group.details {
        let details_html = extract_html_from_details(details);
        if !details_html.is_empty() {
            md.push_str(&html_to_md::convert(&details_html)?);
            md.push_str("\n\n");
        }
    }

    // Filter for functions (element: false, contextual: false)
    let functions: Vec<&FuncContent> = group
        .functions
        .iter()
        .filter(|f| !f.element && !f.contextual)
        .collect();

    if !functions.is_empty() {
        md.push_str("## Functions\n\n");
        for func in functions {
            // Build route from path and name
            let route = if !func.path.is_empty() {
                format!("{}/{}", func.path.join("/"), func.name)
            } else {
                func.name.clone()
            };

            // Fix internal links: /DOCS-BASE/ → ../
            let fixed_route = format!("../{}", route);

            md.push_str(&format!("- [{}]({})", func.title, fixed_route));
            if let Some(oneliner) = &func.oneliner {
                md.push_str(&format!(" - {}", oneliner));
            }
            md.push('\n');
        }
        md.push('\n');
    }

    // Filter for elements (element: true)
    let elements: Vec<&FuncContent> = group
        .functions
        .iter()
        .filter(|f| f.element && !f.contextual)
        .collect();

    if !elements.is_empty() {
        md.push_str("## Elements\n\n");
        for elem in elements {
            let route = if !elem.path.is_empty() {
                format!("{}/{}", elem.path.join("/"), elem.name)
            } else {
                elem.name.clone()
            };

            let fixed_route = format!("../{}", route);

            md.push_str(&format!("- [{}]({})", elem.title, fixed_route));
            if let Some(oneliner) = &elem.oneliner {
                md.push_str(&format!(" - {}", oneliner));
            }
            md.push('\n');
        }
        md.push('\n');
    }

    Ok(md)
}

/// Renders symbols body to Markdown
///
/// Converts a symbol table from docs.json into formatted Markdown with:
/// - Symbol table description
/// - Symbol listings with markup/math shorthands
///
/// # Arguments
///
/// * `content` - JSON value containing symbols definition
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
pub fn render_symbols_body(content: &serde_json::Value) -> Result<String, RenderError> {
    let symbols: SymbolsContent = serde_json::from_value(content.clone())?;

    let mut md = String::new();

    // Symbols description
    if let Some(details) = &symbols.details {
        let details_html = extract_html_from_details(details);
        if !details_html.is_empty() {
            md.push_str(&html_to_md::convert(&details_html)?);
            md.push_str("\n\n");
        }
    }

    // Symbols table
    if !symbols.list.is_empty() {
        md.push_str("## Symbols\n\n");
        md.push_str("| Name | Markup | Math | Unicode |\n");
        md.push_str("|------|--------|------|---------|\n");

        for symbol in &symbols.list {
            let markup = symbol
                .markup_shorthand
                .as_deref()
                .map(|s| format!("`{}`", s))
                .unwrap_or_else(|| "-".to_string());

            let math = symbol
                .math_shorthand
                .as_deref()
                .map(|s| format!("`{}`", s))
                .unwrap_or_else(|| "-".to_string());

            // Format Unicode value: use codepoint if available, otherwise extract from value
            let unicode = if let Some(codepoint) = symbol.codepoint {
                format!("U+{:04X}", codepoint)
            } else if let Some(ref value) = symbol.value {
                value
                    .chars()
                    .next()
                    .map(|c| format!("U+{:04X}", c as u32))
                    .unwrap_or_else(|| "-".to_string())
            } else {
                "-".to_string()
            };

            md.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                symbol.name, markup, math, unicode
            ));
        }
        md.push('\n');
    }

    Ok(md)
}

/// Formats scoped function (method) for type scope
///
/// Helper function used by render_type_body to render methods.
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
    let sig = format_function_signature(func);
    md.push_str(&format!("{}\n\n", sig));

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
    fn test_render_type_basic() {
        let fixture =
            include_str!("../../../../fixtures/typst/v0.12.0/test-fixtures/type-arguments.json");
        let entry: crate::docs::schema::DocsEntry =
            serde_json::from_str(fixture).expect("Failed to parse type-arguments.json");

        let body = entry.body.expect("Entry should have body");
        assert_eq!(body.kind, "type", "Body kind should be type");

        let result = render_type_body(&body.content).expect("Rendering should succeed");

        // Verify constructor section
        assert!(
            result.contains("## Constructor"),
            "Should have Constructor section"
        );

        // Verify methods section
        assert!(result.contains("## Methods"), "Should have Methods section");
    }

    #[test]
    fn test_render_category_basic() {
        let fixture = include_str!(
            "../../../../fixtures/typst/v0.12.0/test-fixtures/category-foundations.json"
        );
        let entry: crate::docs::schema::DocsEntry =
            serde_json::from_str(fixture).expect("Failed to parse category-foundations.json");

        let body = entry.body.expect("Entry should have body");
        assert_eq!(body.kind, "category", "Body kind should be category");

        let result = render_category_body(&body.content).expect("Rendering should succeed");

        // Verify items section
        assert!(result.contains("## Items"), "Should have Items section");
        assert!(result.contains("- ["), "Should have list items");
    }

    #[test]
    fn test_render_group_basic() {
        let fixture =
            include_str!("../../../../fixtures/typst/v0.12.0/test-fixtures/group-calc.json");
        let entry: crate::docs::schema::DocsEntry =
            serde_json::from_str(fixture).expect("Failed to parse group-calc.json");

        let body = entry.body.expect("Entry should have body");
        assert_eq!(body.kind, "group", "Body kind should be group");

        let result = render_group_body(&body.content).expect("Rendering should succeed");

        // Verify functions section
        assert!(
            result.contains("## Functions"),
            "Should have Functions section"
        );
    }

    #[test]
    fn test_render_symbols_basic() {
        let fixture =
            include_str!("../../../../fixtures/typst/v0.12.0/test-fixtures/symbols-emoji.json");
        let entry: crate::docs::schema::DocsEntry =
            serde_json::from_str(fixture).expect("Failed to parse symbols-emoji.json");

        let body = entry.body.expect("Entry should have body");
        assert_eq!(body.kind, "symbols", "Body kind should be symbols");

        let result = render_symbols_body(&body.content).expect("Rendering should succeed");

        // Verify symbols table
        assert!(result.contains("## Symbols"), "Should have Symbols section");
        assert!(
            result.contains("| Name | Markup | Math | Unicode |"),
            "Should have table header"
        );
    }
}

/// Body rendering errors
#[derive(Debug, Error)]
pub enum RenderError {
    /// JSON parsing error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// HTML conversion error
    #[error("HTML conversion error: {0}")]
    HtmlConversionError(#[from] html_to_md::ConversionError),

    /// Function rendering error
    #[error("Function rendering error: {0}")]
    FuncRenderError(#[from] render_func::RenderError),
}
