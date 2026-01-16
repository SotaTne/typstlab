//! Project scaffold creation

use crate::path::has_absolute_or_rooted_component;
use crate::project::builtin_layouts;
use anyhow::{bail, Result};
use chrono::Local;
use std::fs;
use std::path::{Component, Path};

/// Validate project or paper name for security
///
/// # Security
///
/// Blocks:
/// - Absolute paths (e.g., `/tmp/foo`, `C:\Windows`)
/// - Parent directory traversal (`..`)
/// - Current directory (`.`)
/// - Path separators (multiple components like `foo/bar`)
/// - Empty names
/// - Windows drive prefixes (e.g., `C:`)
///
/// Names must be single directory names without path separators.
///
/// # Examples
///
/// ```
/// # use typstlab_core::project::create::validate_name;
/// assert!(validate_name("my-project").is_ok());
/// assert!(validate_name("../../../etc/passwd").is_err());
/// assert!(validate_name("/tmp/malicious").is_err());
/// assert!(validate_name("foo/bar").is_err());
/// ```
pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Name cannot be empty");
    }

    let path = Path::new(name);

    if has_absolute_or_rooted_component(path) {
        bail!("Name cannot be an absolute path: '{}'", name);
    }

    validate_path_components(path, name)?;

    Ok(())
}

/// Validate path components for security
fn validate_path_components(path: &Path, name: &str) -> Result<()> {
    let mut normal_count = 0;

    for component in path.components() {
        match component {
            Component::Normal(_) => normal_count += 1,
            Component::Prefix(_) => bail!("Name cannot contain drive prefix: '{}'", name),
            Component::RootDir => bail!("Name cannot be an absolute path: '{}'", name),
            Component::CurDir => bail!("Name cannot contain current directory (.): '{}'", name),
            Component::ParentDir => {
                bail!("Name cannot contain parent directory (..): '{}'", name)
            }
        }
    }

    if normal_count != 1 {
        bail!(
            "Name must be a single directory name without path separators: '{}'",
            name
        );
    }

    Ok(())
}

/// Create a new project scaffold
///
/// Creates the following structure:
/// - typstlab.toml (project configuration)
/// - .gitignore
/// - papers/ (empty directory for papers)
/// - layouts/ (with builtin layouts copied)
/// - refs/ (empty directory for references)
/// - dist/ (empty directory for build outputs)
/// - rules/ (empty directory for project-level rules)
/// - .typstlab/ (directory for state and cache)
///
/// # Arguments
///
/// * `parent_dir` - Parent directory where project will be created
/// * `project_name` - Name of the project (becomes directory name)
///
/// # Errors
///
/// Returns error if:
/// - Project directory already exists
/// - Directory creation fails
/// - File writing fails
pub fn create_project(parent_dir: &Path, project_name: &str) -> Result<()> {
    // Validate project name for security
    validate_name(project_name)?;

    let project_dir = parent_dir.join(project_name);

    // Check if project already exists
    if project_dir.exists() {
        bail!(
            "Project directory '{}' already exists",
            project_dir.display()
        );
    }

    // Create project root directory
    fs::create_dir(&project_dir)?;

    // Create subdirectories
    fs::create_dir(project_dir.join("papers"))?;
    fs::create_dir(project_dir.join("layouts"))?;
    fs::create_dir(project_dir.join("refs"))?;
    fs::create_dir(project_dir.join("dist"))?;
    fs::create_dir(project_dir.join("rules"))?;
    fs::create_dir(project_dir.join(".typstlab"))?;

    // Copy builtin layouts to layouts/
    copy_builtin_layouts(&project_dir.join("layouts"))?;

    // Create typstlab.toml
    create_typstlab_toml(&project_dir, project_name)?;

    // Create .gitignore
    create_gitignore(&project_dir)?;

    Ok(())
}

/// Copy builtin layouts to project layouts/ directory
fn copy_builtin_layouts(layouts_dir: &Path) -> Result<()> {
    // Get builtin layouts
    let default_layout = builtin_layouts::get_builtin_layout("default")
        .ok_or_else(|| anyhow::anyhow!("Builtin 'default' layout not found"))?;

    let minimal_layout = builtin_layouts::get_builtin_layout("minimal")
        .ok_or_else(|| anyhow::anyhow!("Builtin 'minimal' layout not found"))?;

    // Create layout directories
    let default_dir = layouts_dir.join("default");
    let minimal_dir = layouts_dir.join("minimal");

    fs::create_dir_all(&default_dir)?;
    fs::create_dir_all(&minimal_dir)?;

    // Write default layout files
    if let Some(meta) = default_layout.meta_template {
        fs::write(default_dir.join("meta.tmp.typ"), meta)?;
    }
    if let Some(header) = default_layout.header_static {
        fs::write(default_dir.join("header.typ"), header)?;
    }
    if let Some(refs) = default_layout.refs_template {
        fs::write(default_dir.join("refs.tmp.typ"), refs)?;
    }

    // Write minimal layout files
    if let Some(meta) = minimal_layout.meta_template {
        fs::write(minimal_dir.join("meta.tmp.typ"), meta)?;
    }
    if let Some(refs) = minimal_layout.refs_template {
        fs::write(minimal_dir.join("refs.tmp.typ"), refs)?;
    }

    Ok(())
}

/// Create typstlab.toml with project configuration
fn create_typstlab_toml(project_dir: &Path, project_name: &str) -> Result<()> {
    let today = Local::now().format("%Y-%m-%d").to_string();

    let content = format!(
        r#"[project]
name = "{}"
init_date = "{}"

[typst]
version = "0.12.0"

# Uncomment to specify default layout for all papers
# [project.defaults]
# layout = "default"
"#,
        project_name, today
    );

    fs::write(project_dir.join("typstlab.toml"), content)?;
    Ok(())
}

/// Create .gitignore with common ignores
fn create_gitignore(project_dir: &Path) -> Result<()> {
    let content = r#"# Build outputs
dist/
*.pdf

# Typstlab state
.typstlab/state.json
.typstlab/typst-*/

# Process lock files (temporary, auto-cleaned on exit)
.typstlab/*.lock
.typstlab/kb/*.lock
.typstlab/locks/*.lock

# Typst cache
.typst-cache/

# OS files
.DS_Store
Thumbs.db

# Editor files
*.swp
*.swo
*~
.vscode/
.idea/

# Temporary files
*.tmp
.tmp/
"#;

    fs::write(project_dir.join(".gitignore"), content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_create_project_success() {
        let temp = TempDir::new().unwrap();
        let result = create_project(temp.path(), "test-project");
        assert!(result.is_ok());

        let project_dir = temp.path().join("test-project");
        assert!(project_dir.exists());
        assert!(project_dir.join("typstlab.toml").exists());
        assert!(project_dir.join(".gitignore").exists());
        assert!(project_dir.join("papers").is_dir());
        assert!(project_dir.join("layouts").is_dir());
        assert!(project_dir.join("refs").is_dir());
        assert!(project_dir.join("dist").is_dir());
        assert!(project_dir.join("rules").is_dir());
        assert!(project_dir.join(".typstlab").is_dir());
    }

    #[test]
    fn test_create_project_copies_builtin_layouts() {
        let temp = TempDir::new().unwrap();
        create_project(temp.path(), "test-project").unwrap();

        let layouts_dir = temp.path().join("test-project/layouts");
        assert!(layouts_dir.join("default").is_dir());
        assert!(layouts_dir.join("minimal").is_dir());

        // Check default layout files
        assert!(layouts_dir.join("default/meta.tmp.typ").exists());
        assert!(layouts_dir.join("default/header.typ").exists());
        assert!(layouts_dir.join("default/refs.tmp.typ").exists());

        // Check minimal layout files
        assert!(layouts_dir.join("minimal/meta.tmp.typ").exists());
        assert!(layouts_dir.join("minimal/refs.tmp.typ").exists());
    }

    #[test]
    fn test_create_project_valid_toml() {
        let temp = TempDir::new().unwrap();
        create_project(temp.path(), "my-project").unwrap();

        let toml_path = temp.path().join("my-project/typstlab.toml");
        let content = fs::read_to_string(toml_path).unwrap();

        assert!(content.contains("[project]"));
        assert!(content.contains("name = \"my-project\""));
        assert!(content.contains("[typst]"));
        assert!(content.contains("version = \"0.12.0\""));
    }

    #[test]
    fn test_create_project_valid_gitignore() {
        let temp = TempDir::new().unwrap();
        create_project(temp.path(), "test-project").unwrap();

        let gitignore_path = temp.path().join("test-project/.gitignore");
        let content = fs::read_to_string(gitignore_path).unwrap();

        assert!(content.contains("dist/"));
        assert!(content.contains("*.pdf"));
        assert!(content.contains(".typstlab/"));
    }

    #[test]
    fn test_create_project_fails_if_exists() {
        let temp = TempDir::new().unwrap();

        // Create first time - should succeed
        create_project(temp.path(), "test-project").unwrap();

        // Create second time - should fail
        let result = create_project(temp.path(), "test-project");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_validate_name_accepts_valid_names() {
        assert!(validate_name("my-project").is_ok());
        assert!(validate_name("paper1").is_ok());
        assert!(validate_name("test_123").is_ok());
        assert!(validate_name("foo-bar-baz").is_ok());
    }

    #[test]
    fn test_validate_name_rejects_empty() {
        let result = validate_name("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validate_name_rejects_parent_dir() {
        let result = validate_name("../etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("parent directory"));

        let result = validate_name("foo/../bar");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_name_rejects_absolute_path() {
        let result = validate_name("/tmp/malicious");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("absolute path"));
    }

    #[test]
    fn test_validate_name_rejects_current_dir() {
        let result = validate_name("./foo");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("current directory"));
    }

    #[test]
    fn test_validate_name_rejects_path_separators() {
        let result = validate_name("foo/bar");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("path separators"));

        let result = validate_name("foo/bar/baz");
        assert!(result.is_err());

        let result = validate_name("nested/dir/name");
        assert!(result.is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_validate_name_rejects_windows_drive() {
        let result = validate_name(r"C:\Windows");
        assert!(result.is_err(), "Should reject Windows drive prefix");

        // More flexible assertion: check for either error message
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("drive") || err_msg.contains("absolute"),
            "Error should mention drive or absolute, got: {}",
            err_msg
        );
    }
}
