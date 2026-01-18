//! YAML frontmatter generation for documentation pages

use serde::Serialize;
use thiserror::Error;

/// YAML frontmatter for documentation pages
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct DocsFrontmatter {
    /// Page title
    pub title: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Generates YAML frontmatter string with delimiters
///
/// Creates a complete YAML frontmatter block:
/// ```yaml
/// ---
/// title: Page Title
/// description: |
///   Optional description text
/// ---
/// ```
///
/// # Arguments
///
/// * `title` - Page title
/// * `description` - Optional description text
///
/// # Returns
///
/// Complete YAML frontmatter string with `---` delimiters
///
/// # Errors
///
/// Returns error if YAML serialization fails
///
/// # Examples
///
/// ```
/// use typstlab_typst::docs::generate::generate_frontmatter;
///
/// let yaml = generate_frontmatter("My Page", Some("A description")).unwrap();
/// assert!(yaml.starts_with("---\n"));
/// assert!(yaml.contains("title: My Page"));
/// ```
pub fn generate_frontmatter(
    title: &str,
    description: Option<&str>,
) -> Result<String, FrontmatterError> {
    let frontmatter = DocsFrontmatter {
        title: title.to_string(),
        description: description.map(|s| s.to_string()),
    };

    // Serialize to YAML (without delimiters)
    let yaml_content = serde_yaml::to_string(&frontmatter)
        .map_err(|e| FrontmatterError::YamlSerialization(e.to_string()))?;

    // Add YAML delimiters
    Ok(format!("---\n{}---\n\n", yaml_content))
}

/// Frontmatter generation errors
#[derive(Debug, Error)]
pub enum FrontmatterError {
    /// YAML serialization error
    #[error("YAML serialization error: {0}")]
    YamlSerialization(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_frontmatter_title_only() {
        let result = generate_frontmatter("My Page", None).expect("Should generate frontmatter");

        assert!(
            result.starts_with("---\n"),
            "Should start with YAML delimiter"
        );
        assert!(
            result.contains("title: My Page"),
            "Should contain title field"
        );
        assert!(result.ends_with("---\n\n"), "Should end with delimiter");
        assert!(
            !result.contains("description:"),
            "Should not have description field"
        );
    }

    #[test]
    fn test_generate_frontmatter_with_description() {
        let result = generate_frontmatter("Overview", Some("A detailed description"))
            .expect("Should generate frontmatter");

        assert!(result.starts_with("---\n"));
        assert!(result.contains("title: Overview"));
        assert!(result.contains("description:"));
        assert!(result.contains("A detailed description"));
        assert!(result.ends_with("---\n\n"));
    }

    #[test]
    fn test_generate_frontmatter_multiline_description() {
        let desc = "Line 1\nLine 2\nLine 3";
        let result = generate_frontmatter("Test", Some(desc)).expect("Should generate frontmatter");

        assert!(result.contains("description: |"));
        assert!(result.contains("Line 1"));
        assert!(result.contains("Line 2"));
        assert!(result.contains("Line 3"));
    }

    #[test]
    fn test_frontmatter_struct_serialization() {
        let frontmatter = DocsFrontmatter {
            title: "Test Title".to_string(),
            description: Some("Test description".to_string()),
        };

        let yaml = serde_yaml::to_string(&frontmatter).expect("Should serialize");

        assert!(yaml.contains("title: Test Title"));
        assert!(yaml.contains("description:"));
    }

    #[test]
    fn test_frontmatter_struct_no_description() {
        let frontmatter = DocsFrontmatter {
            title: "Test Title".to_string(),
            description: None,
        };

        let yaml = serde_yaml::to_string(&frontmatter).expect("Should serialize");

        assert!(yaml.contains("title: Test Title"));
        // serde's skip_serializing_if should omit description
        assert!(!yaml.contains("description:"));
    }
}
