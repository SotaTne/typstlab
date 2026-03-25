//! Template resolution for template generation

use crate::TypstlabError;
use std::path::{Path, PathBuf};

/// Template structure holding arbitrary files
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    /// Theme name (corresponds to directory in templates/)
    pub theme: String,
    /// Files in the template (relative path -> content)
    pub files: Vec<(PathBuf, String)>,
}

impl Template {
    /// Create a new template with theme name
    pub fn new(theme: impl Into<String>) -> Self {
        Self {
            theme: theme.into(),
            files: Vec::new(),
        }
    }

    /// Add a file to the template
    pub fn with_file(mut self, path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        self.files.push((path.into(), content.into()));
        self
    }
}

/// Resolve template by name
///
/// Resolution order:
/// 1. User template in `<root>/templates/<name>/`
/// 2. Builtin template
///
/// Returns error if template not found in either location.
pub fn resolve_template(root: &Path, name: &str) -> Result<Template, TypstlabError> {
    // Try user template first
    let user_template_dir = root.join("templates").join(name);
    if user_template_dir.exists() && user_template_dir.is_dir() {
        return load_template_from_dir(&user_template_dir, name);
    }

    // Try builtin template
    super::builtin_templates::get_builtin_template(name)
        .ok_or_else(|| TypstlabError::TemplateNotFound(name.to_string()))
}

/// Load template from a directory
fn load_template_from_dir(dir: &Path, name: &str) -> Result<Template, TypstlabError> {
    let mut template = Template::new(name);

    // Recursively discover all files
    load_files_recursive(dir, dir, &mut template)?;

    Ok(template)
}

fn load_files_recursive(
    base: &Path,
    current: &Path,
    template: &mut Template,
) -> Result<(), TypstlabError> {
    for entry in std::fs::read_dir(current).map_err(TypstlabError::IoError)? {
        let entry = entry.map_err(TypstlabError::IoError)?;
        let path = entry.path();

        if path.is_dir() {
            load_files_recursive(base, &path, template)?;
        } else {
            let relative_path = path.strip_prefix(base).map_err(|_| {
                TypstlabError::Generic(format!(
                    "Failed to calculate relative path for {}",
                    path.display()
                ))
            })?;

            let content = std::fs::read_to_string(&path).map_err(|e| {
                TypstlabError::TemplateInvalid(format!(
                    "Failed to read template file at {}: {}",
                    path.display(),
                    e
                ))
            })?;

            template.files.push((relative_path.to_path_buf(), content));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use typstlab_testkit::temp_dir_in_workspace;

    #[test]
    fn test_template_builder() {
        let template = Template::new("test")
            .with_file("main.typ", "main content")
            .with_file("template.tmp.typ", "tpl content");

        assert_eq!(template.theme, "test");
        assert_eq!(template.files.len(), 2);
        assert_eq!(template.files[0].0, PathBuf::from("main.typ"));
        assert_eq!(template.files[0].1, "main content");
    }

    #[test]
    fn test_resolve_builtin_default_template() {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        let template = resolve_template(root, "default").unwrap();
        assert_eq!(template.theme, "default");
        assert!(!template.files.is_empty());
    }

    #[test]
    fn test_load_template_from_dir_recursive() {
        let temp = temp_dir_in_workspace();
        let dir = temp.path().join("test_template");
        fs::create_dir_all(dir.join("sub")).unwrap();

        fs::write(dir.join("main.typ"), "main content").unwrap();
        fs::write(dir.join("sub/lib.typ"), "lib content").unwrap();

        let template = load_template_from_dir(&dir, "test").unwrap();
        assert_eq!(template.files.len(), 2);

        let paths: Vec<PathBuf> = template.files.iter().map(|(p, _)| p.clone()).collect();
        assert!(paths.contains(&PathBuf::from("main.typ")));
        assert!(paths.contains(&PathBuf::from("sub/lib.typ")));
    }
}
