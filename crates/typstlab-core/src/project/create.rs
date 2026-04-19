//! Project scaffold creation

use crate::path::has_absolute_or_rooted_component;
use crate::project::builtin_templates;
use anyhow::{bail, Result};
use chrono::Local;
use std::fs;
use std::path::Path;

/// Create a new project scaffold
///
/// Creates the following structure:
/// - typstlab.toml (project configuration)
/// - .gitignore (common ignores)
/// - papers/ (empty directory for papers)
/// - templates/ (with builtin templates)
/// - refs/ (empty directory for common references)
/// - dist/ (empty directory for build outputs)
/// - .typstlab/ (private directory for state and cache)
///
/// # Arguments
///
/// * `root` - Directory to create the project in
/// * `project_name` - Name of the project (becomes directory name if initialized)
///
/// # Errors
///
/// Returns error if:
/// - Project directory already exists
/// - Directory creation fails
/// - File writing fails
pub fn create_project(root: &Path, project_name: &str) -> Result<()> {
    let project_dir = root.join(project_name);

    // Check if project already exists
    if project_dir.exists() {
        bail!(
            "Project directory '{}' already exists",
            project_dir.display()
        );
    }

    // Create directory structure
    fs::create_dir_all(&project_dir)?;
    fs::create_dir_all(project_dir.join("papers"))?;
    fs::create_dir_all(project_dir.join("templates"))?;
    fs::create_dir_all(project_dir.join("refs"))?;
    fs::create_dir_all(project_dir.join("dist"))?;
    fs::create_dir_all(project_dir.join(".typstlab"))?;

    // Copy builtin templates to templates/
    copy_builtin_templates(&project_dir.join("templates"))?;

    // Create typstlab.toml
    create_typstlab_toml(&project_dir, project_name)?;

    // Create .gitignore (only if not exists to avoid overwriting user config)
    if !project_dir.join(".gitignore").exists() {
        create_gitignore(&project_dir)?;
    }

    Ok(())
}

/// Initialize a project in an existing directory
///
/// Similar to create_project, but works in the provided directory directly.
pub fn init_project(target_dir: &Path) -> Result<()> {
    // Check if already initialized
    if target_dir.join("typstlab.toml").exists() {
        bail!("Project already initialized at '{}'", target_dir.display());
    }

    // Determine project name from directory name
    let project_name = target_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("typstlab-project");

    // Create directory structure if they don't exist
    fs::create_dir_all(target_dir.join("papers"))?;
    fs::create_dir_all(target_dir.join("templates"))?;
    fs::create_dir_all(target_dir.join("refs"))?;
    fs::create_dir_all(target_dir.join("dist"))?;
    fs::create_dir_all(target_dir.join(".typstlab"))?;

    // Copy builtin templates to templates/
    copy_builtin_templates(&target_dir.join("templates"))?;

    // Create typstlab.toml (if not exists)
    let config_path = target_dir.join("typstlab.toml");
    if !config_path.exists() {
        create_typstlab_toml(target_dir, project_name)?;
    }

    // Create .gitignore (only if not exists)
    if !target_dir.join(".gitignore").exists() {
        create_gitignore(target_dir)?;
    }

    Ok(())
}

/// Copy builtin templates to project templates/ directory
fn copy_builtin_templates(templates_dir: &Path) -> Result<()> {
    // Only 'default' template is used as per user request
    if let Some(template) = builtin_templates::get_builtin_template("default") {
        let template_dir = templates_dir.join("default");
        fs::create_dir_all(&template_dir)?;

        for (rel_path, content) in template.files {
            let full_path = template_dir.join(rel_path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(full_path, content)?;
        }
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

# Uncomment to specify default template for all papers
# [project.defaults]
# template = "default"
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

/// Validate if the provided name is safe for use as paper ID or project name.
pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Name cannot be empty");
    }

    // Block path traversal and special directories
    if name == "." || name == ".." {
        bail!("Name cannot be current or parent directory");
    }

    // Block path separators
    if name.contains('/') || name.contains('\\') {
        bail!("Name cannot contain path separators");
    }

    // Block absolute paths
    let path = Path::new(name);
    if has_absolute_or_rooted_component(path) {
        bail!("Name cannot be an absolute path");
    }

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
        assert!(project_dir.join("templates").is_dir());
        assert!(project_dir.join("refs").is_dir());
        assert!(project_dir.join("dist").is_dir());
        assert!(project_dir.join(".typstlab").is_dir());
    }

    #[test]
    fn test_create_project_copies_builtin_templates() {
        let temp = TempDir::new().unwrap();
        create_project(temp.path(), "test-project").unwrap();

        let templates_dir = temp.path().join("test-project/templates");
        assert!(templates_dir.join("default").is_dir());

        // Only default is created now
        assert!(!templates_dir.join("minimal").exists());

        // Check default template files
        assert!(templates_dir.join("default/main.tmp.typ").exists());
        assert!(templates_dir.join("default/template.typ").exists());
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
    fn test_validate_name_accepts_valid_names() {
        assert!(validate_name("my-project").is_ok());
        assert!(validate_name("paper1").is_ok());
    }

    #[test]
    fn test_validate_name_rejects_empty() {
        assert!(validate_name("").is_err());
    }
}
