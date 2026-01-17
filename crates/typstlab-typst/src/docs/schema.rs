//! Typst documentation JSON schema with validation and error recovery
//!
//! Handles docs.json schema evolution across Typst versions with tolerant parsing.
//! Supports both v0.12.0 and v0.13.0 format variations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Component, Path};
use thiserror::Error;

/// Documentation entry in docs.json
///
/// Represents a documentation page or section. Recursive structure allows
/// nested documentation hierarchies.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct DocsEntry {
    /// Route path (e.g., "/DOCS-BASE/tutorial/writing-in-typst/")
    pub route: String,

    /// Human-readable title
    pub title: String,

    /// Optional description
    #[serde(default)]
    pub description: Option<String>,

    /// Optional part identifier
    #[serde(default)]
    pub part: Option<String>,

    /// Table of contents outline
    #[serde(default)]
    pub outline: Vec<OutlineItem>,

    /// Page content body
    #[serde(default)]
    pub body: Option<Body>,

    /// Child pages
    #[serde(default)]
    pub children: Vec<DocsEntry>,

    /// Tolerant: Accept unknown fields without failing (for schema evolution)
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Documentation body content
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Body {
    /// Content type (e.g., "html")
    pub kind: String,

    /// Actual content
    ///
    /// Can be either:
    /// - String: HTML content (for tutorial pages, guides)
    /// - Object: Function/type definition (for reference pages)
    pub content: serde_json::Value,
}

impl Body {
    /// Checks if content is HTML string
    pub fn is_html(&self) -> bool {
        self.content.is_string()
    }

    /// Checks if content is function/type definition object
    pub fn is_definition(&self) -> bool {
        self.content.is_object()
    }

    /// Gets content as HTML string
    ///
    /// # Errors
    ///
    /// Returns error if content is not a string
    pub fn as_html(&self) -> Result<&str, SchemaError> {
        self.content
            .as_str()
            .ok_or_else(|| SchemaError::InvalidContentType("Expected HTML string".to_string()))
    }
}

/// Table of contents outline item
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct OutlineItem {
    /// Anchor ID
    pub id: String,

    /// Display name
    pub name: String,

    /// Nested outline items
    #[serde(default)]
    pub children: Vec<OutlineItem>,
}

impl DocsEntry {
    /// Validates docs entry for security and correctness
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Route is empty
    /// - Title is empty
    /// - Route contains absolute or rooted path components
    /// - Route contains parent directory traversal (`..`)
    pub fn validate(&self) -> Result<(), SchemaError> {
        // Route must not be empty
        if self.route.is_empty() {
            return Err(SchemaError::MissingRoute);
        }

        // Title must not be empty
        if self.title.is_empty() {
            return Err(SchemaError::MissingTitle);
        }

        // Validate route for path traversal
        let route_path = Path::new(&self.route);

        // Check for absolute or rooted paths (cross-platform)
        if typstlab_core::path::has_absolute_or_rooted_component(route_path) {
            return Err(SchemaError::AbsolutePath(self.route.clone()));
        }

        // Check for parent directory traversal (..)
        if route_path
            .components()
            .any(|c| matches!(c, Component::ParentDir))
        {
            return Err(SchemaError::PathTraversal(self.route.clone()));
        }

        Ok(())
    }
}

/// Schema validation errors
#[derive(Debug, Error, PartialEq)]
pub enum SchemaError {
    /// Missing route field
    #[error("Missing route in docs entry")]
    MissingRoute,

    /// Missing title field
    #[error("Missing title in docs entry")]
    MissingTitle,

    /// Absolute or rooted path not allowed
    #[error("Absolute or rooted path not allowed: {0}")]
    AbsolutePath(String),

    /// Path traversal (..) not allowed
    #[error("Path traversal (..) not allowed: {0}")]
    PathTraversal(String),

    /// JSON parse error
    #[error("JSON parse error: {0}")]
    ParseError(String),

    /// Invalid content type
    #[error("Invalid content type: {0}")]
    InvalidContentType(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test: Parse valid docs.json entry
    #[test]
    fn test_parse_valid_entry() {
        let json = r#"{
            "route": "/DOCS-BASE/tutorial/",
            "title": "Tutorial",
            "description": "Learn Typst",
            "part": null,
            "outline": [],
            "body": null,
            "children": []
        }"#;

        let entry: DocsEntry = serde_json::from_str(json).expect("Should parse valid entry");
        assert_eq!(entry.route, "/DOCS-BASE/tutorial/");
        assert_eq!(entry.title, "Tutorial");
        assert_eq!(entry.description, Some("Learn Typst".to_string()));
        assert_eq!(entry.outline.len(), 0);
        assert!(entry.body.is_none());
        assert_eq!(entry.children.len(), 0);
    }

    /// Test: Parse entry with body
    #[test]
    fn test_parse_entry_with_body() {
        let json = r#"{
            "route": "/DOCS-BASE/",
            "title": "Overview",
            "body": {
                "kind": "html",
                "content": "<p>Hello</p>"
            },
            "children": []
        }"#;

        let entry: DocsEntry = serde_json::from_str(json).expect("Should parse entry with body");
        let body = entry.body.expect("Should have body");
        assert_eq!(body.kind, "html");
        assert_eq!(body.content, "<p>Hello</p>");
    }

    /// Test: Parse entry with outline
    #[test]
    fn test_parse_entry_with_outline() {
        let json = r#"{
            "route": "/DOCS-BASE/tutorial/",
            "title": "Tutorial",
            "outline": [
                {
                    "id": "when-typst",
                    "name": "When Typst",
                    "children": []
                }
            ],
            "children": []
        }"#;

        let entry: DocsEntry = serde_json::from_str(json).expect("Should parse entry with outline");
        assert_eq!(entry.outline.len(), 1);
        assert_eq!(entry.outline[0].id, "when-typst");
        assert_eq!(entry.outline[0].name, "When Typst");
    }

    /// Test: Parse entry with nested children
    #[test]
    fn test_parse_entry_with_children() {
        let json = r#"{
            "route": "/DOCS-BASE/tutorial/",
            "title": "Tutorial",
            "children": [
                {
                    "route": "/DOCS-BASE/tutorial/writing/",
                    "title": "Writing",
                    "children": []
                }
            ]
        }"#;

        let entry: DocsEntry =
            serde_json::from_str(json).expect("Should parse entry with children");
        assert_eq!(entry.children.len(), 1);
        assert_eq!(entry.children[0].route, "/DOCS-BASE/tutorial/writing/");
        assert_eq!(entry.children[0].title, "Writing");
    }

    /// Test: Tolerate unknown fields (schema evolution)
    #[test]
    fn test_tolerant_parsing_unknown_fields() {
        let json = r#"{
            "route": "/DOCS-BASE/",
            "title": "Overview",
            "new_field_v013": "future value",
            "another_unknown": 42,
            "children": []
        }"#;

        let entry: DocsEntry = serde_json::from_str(json).expect("Should tolerate unknown fields");
        assert_eq!(entry.route, "/DOCS-BASE/");
        assert_eq!(entry.title, "Overview");

        // Unknown fields stored in extra
        assert!(entry.extra.contains_key("new_field_v013"));
        assert!(entry.extra.contains_key("another_unknown"));
    }

    /// Test: Validation - valid entry passes
    #[test]
    fn test_validation_valid_entry() {
        let entry = DocsEntry {
            route: "tutorial/writing/".to_string(),
            title: "Writing".to_string(),
            description: None,
            part: None,
            outline: vec![],
            body: None,
            children: vec![],
            extra: HashMap::new(),
        };

        assert!(entry.validate().is_ok());
    }

    /// Test: Validation - empty route rejected
    #[test]
    fn test_validation_empty_route() {
        let entry = DocsEntry {
            route: "".to_string(),
            title: "Title".to_string(),
            description: None,
            part: None,
            outline: vec![],
            body: None,
            children: vec![],
            extra: HashMap::new(),
        };

        let result = entry.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SchemaError::MissingRoute);
    }

    /// Test: Validation - empty title rejected
    #[test]
    fn test_validation_empty_title() {
        let entry = DocsEntry {
            route: "tutorial/".to_string(),
            title: "".to_string(),
            description: None,
            part: None,
            outline: vec![],
            body: None,
            children: vec![],
            extra: HashMap::new(),
        };

        let result = entry.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SchemaError::MissingTitle);
    }

    /// Test: Validation - absolute path rejected
    #[test]
    fn test_validation_absolute_path() {
        let entry = DocsEntry {
            route: "/tmp/malicious".to_string(),
            title: "Malicious".to_string(),
            description: None,
            part: None,
            outline: vec![],
            body: None,
            children: vec![],
            extra: HashMap::new(),
        };

        let result = entry.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            SchemaError::AbsolutePath(path) => assert_eq!(path, "/tmp/malicious"),
            e => panic!("Expected AbsolutePath error, got: {:?}", e),
        }
    }

    /// Test: Validation - parent directory traversal rejected
    #[test]
    fn test_validation_parent_traversal() {
        let entry = DocsEntry {
            route: "../../../etc/passwd".to_string(),
            title: "Malicious".to_string(),
            description: None,
            part: None,
            outline: vec![],
            body: None,
            children: vec![],
            extra: HashMap::new(),
        };

        let result = entry.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            SchemaError::PathTraversal(path) => assert_eq!(path, "../../../etc/passwd"),
            e => panic!("Expected PathTraversal error, got: {:?}", e),
        }
    }

    /// Test: Parse top-level docs.json array
    #[test]
    fn test_parse_docs_json_array() {
        let json = r#"[
            {
                "route": "/DOCS-BASE/",
                "title": "Overview",
                "children": []
            },
            {
                "route": "/DOCS-BASE/tutorial/",
                "title": "Tutorial",
                "children": []
            }
        ]"#;

        let entries: Vec<DocsEntry> =
            serde_json::from_str(json).expect("Should parse docs.json array");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].route, "/DOCS-BASE/");
        assert_eq!(entries[1].route, "/DOCS-BASE/tutorial/");
    }

    /// Test: Malformed JSON returns parse error
    #[test]
    fn test_malformed_json() {
        let json = r#"{
            "route": "/DOCS-BASE/",
            "title": "Overview"
            "missing comma here"
        }"#;

        let result: Result<DocsEntry, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
