//! Layout resolution for template generation

use crate::TypstlabError;
use std::path::Path;

/// Layout structure holding template and static files
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Theme name (corresponds to directory in layouts/)
    pub theme: String,
    /// meta.tmp.typ template content (if exists)
    pub meta_template: Option<String>,
    /// header.typ static content (if exists)
    pub header_static: Option<String>,
    /// refs.tmp.typ template content (if exists)
    pub refs_template: Option<String>,
}

impl Layout {
    /// Create a new layout with theme name
    pub fn new(theme: impl Into<String>) -> Self {
        Self {
            theme: theme.into(),
            meta_template: None,
            header_static: None,
            refs_template: None,
        }
    }

    /// Set meta template
    pub fn with_meta_template(mut self, content: impl Into<String>) -> Self {
        self.meta_template = Some(content.into());
        self
    }

    /// Set header static
    pub fn with_header_static(mut self, content: impl Into<String>) -> Self {
        self.header_static = Some(content.into());
        self
    }

    /// Set refs template
    pub fn with_refs_template(mut self, content: impl Into<String>) -> Self {
        self.refs_template = Some(content.into());
        self
    }
}

/// Resolve layout by name
///
/// Resolution order:
/// 1. User layout in `<root>/layouts/<name>/`
/// 2. Builtin layout
///
/// Returns error if layout not found in either location.
pub fn resolve_layout(root: &Path, name: &str) -> Result<Layout, TypstlabError> {
    // Try user layout first
    let user_layout_dir = root.join("layouts").join(name);
    if user_layout_dir.exists() && user_layout_dir.is_dir() {
        return load_layout_from_dir(&user_layout_dir, name);
    }

    // Try builtin layout
    super::builtin_layouts::get_builtin_layout(name)
        .ok_or_else(|| TypstlabError::LayoutNotFound(name.to_string()))
}

/// Load layout from a directory
fn load_layout_from_dir(dir: &Path, name: &str) -> Result<Layout, TypstlabError> {
    let mut layout = Layout::new(name);

    // Load meta.tmp.typ if exists
    let meta_path = dir.join("meta.tmp.typ");
    if meta_path.is_file() {
        let content = std::fs::read_to_string(&meta_path).map_err(|e| {
            TypstlabError::LayoutInvalid(format!(
                "Failed to read meta.tmp.typ at {}: {}",
                meta_path.display(),
                e
            ))
        })?;
        layout.meta_template = Some(content);
    }

    // Load header.typ if exists
    let header_path = dir.join("header.typ");
    if header_path.is_file() {
        let content = std::fs::read_to_string(&header_path).map_err(|e| {
            TypstlabError::LayoutInvalid(format!(
                "Failed to read header.typ at {}: {}",
                header_path.display(),
                e
            ))
        })?;
        layout.header_static = Some(content);
    }

    // Load refs.tmp.typ if exists
    let refs_path = dir.join("refs.tmp.typ");
    if refs_path.is_file() {
        let content = std::fs::read_to_string(&refs_path).map_err(|e| {
            TypstlabError::LayoutInvalid(format!(
                "Failed to read refs.tmp.typ at {}: {}",
                refs_path.display(),
                e
            ))
        })?;
        layout.refs_template = Some(content);
    }

    Ok(layout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use typstlab_testkit::temp_dir_in_workspace;

    #[test]
    fn test_layout_builder() {
        let layout = Layout::new("test")
            .with_meta_template("meta content")
            .with_header_static("header content")
            .with_refs_template("refs content");

        assert_eq!(layout.theme, "test");
        assert_eq!(layout.meta_template.as_deref(), Some("meta content"));
        assert_eq!(layout.header_static.as_deref(), Some("header content"));
        assert_eq!(layout.refs_template.as_deref(), Some("refs content"));
    }

    #[test]
    fn test_resolve_builtin_default_layout() {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        let layout = resolve_layout(root, "default").unwrap();
        assert_eq!(layout.theme, "default");
        assert!(layout.meta_template.is_some());
        assert!(layout.header_static.is_some());
        assert!(layout.refs_template.is_some());
    }

    #[test]
    fn test_resolve_builtin_minimal_layout() {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        let layout = resolve_layout(root, "minimal").unwrap();
        assert_eq!(layout.theme, "minimal");
        assert!(layout.meta_template.is_some());
        assert!(layout.refs_template.is_some());
    }

    #[test]
    fn test_resolve_nonexistent_layout() {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        let result = resolve_layout(root, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_user_layout_priority() {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Create user layout directory
        let layouts_dir = root.join("layouts").join("default");
        fs::create_dir_all(&layouts_dir).unwrap();

        // Create custom meta.tmp.typ
        fs::write(
            layouts_dir.join("meta.tmp.typ"),
            "= Custom Default Layout\n",
        )
        .unwrap();

        let layout = resolve_layout(root, "default").unwrap();
        assert_eq!(layout.theme, "default");
        assert_eq!(
            layout.meta_template.as_deref(),
            Some("= Custom Default Layout\n")
        );
    }

    #[test]
    fn test_load_layout_from_dir_with_all_files() {
        let temp = temp_dir_in_workspace();
        let dir = temp.path().join("test_layout");
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("meta.tmp.typ"), "meta content").unwrap();
        fs::write(dir.join("header.typ"), "header content").unwrap();
        fs::write(dir.join("refs.tmp.typ"), "refs content").unwrap();

        let layout = load_layout_from_dir(&dir, "test").unwrap();
        assert_eq!(layout.meta_template.as_deref(), Some("meta content"));
        assert_eq!(layout.header_static.as_deref(), Some("header content"));
        assert_eq!(layout.refs_template.as_deref(), Some("refs content"));
    }

    #[test]
    fn test_load_layout_from_dir_partial_files() {
        let temp = temp_dir_in_workspace();
        let dir = temp.path().join("test_layout");
        fs::create_dir_all(&dir).unwrap();

        // Only create meta.tmp.typ
        fs::write(dir.join("meta.tmp.typ"), "meta only").unwrap();

        let layout = load_layout_from_dir(&dir, "test").unwrap();
        assert_eq!(layout.meta_template.as_deref(), Some("meta only"));
        assert!(layout.header_static.is_none());
        assert!(layout.refs_template.is_none());
    }

    #[test]
    fn test_load_layout_from_dir_empty() {
        let temp = temp_dir_in_workspace();
        let dir = temp.path().join("test_layout");
        fs::create_dir_all(&dir).unwrap();

        let layout = load_layout_from_dir(&dir, "test").unwrap();
        assert!(layout.meta_template.is_none());
        assert!(layout.header_static.is_none());
        assert!(layout.refs_template.is_none());
    }
}
