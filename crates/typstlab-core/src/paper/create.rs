//! Paper scaffold creation

use crate::project::{validate_name, Project};
use crate::template::{TemplateContext, TemplateEngine};
use anyhow::{bail, Result};
use chrono::Local;
use std::fs;
use std::path::Path;

/// Create a new paper scaffold in the project
///
/// Creates the following structure:
/// - papers/<paper_id>/
///   - paper.toml (paper configuration)
///   - main.typ (main Typst file)
///   - sections/ (empty directory for paper sections)
///   - assets/ (empty directory for images, etc.)
///   - rules/ (directory for paper-specific rules, with README.md)
///
/// # Arguments
///
/// * `project` - Project context (for accessing root and config)
/// * `paper_id` - ID of the paper (becomes directory name)
///
/// # Errors
///
/// Returns error if:
/// - Paper directory already exists
/// - Directory creation fails
/// - File writing fails
pub fn create_paper<F>(
    project: &Project,
    paper_id: &str,
    title: Option<String>,
    template: Option<String>,
    init_remote: Option<F>,
) -> Result<()>
where
    F: FnOnce(&str, &Path) -> Result<()>,
{
    // Validate paper ID for security
    validate_name(paper_id)?;

    let papers_dir = project.root.join("papers");
    let paper_dir = papers_dir.join(paper_id);

    // Check if paper already exists
    if paper_dir.exists() {
        bail!("Paper '{}' already exists", paper_id);
    }

    // Handle template if provided
    if let Some(template_name) = template {
        let template_dir = project.root.join("templates").join(&template_name);

        if template_dir.exists() {
            let paper_title = title.clone().unwrap_or_else(|| paper_id.to_string());
            fs::create_dir_all(&paper_dir)?;

            // Prepare template context
            let today = Local::now().format("%Y-%m-%d").to_string();
            let context_data = toml::from_str(&format!(
                r#"[paper]
id = "{}"
title = "{}"
date = "{}"
language = "en"
authors = []
"#,
                paper_id, paper_title, today
            ))?;
            let context = TemplateContext::new(context_data);

            expand_local_template(&template_dir, &paper_dir, &context)?;
        } else if let Some(init_fn) = init_remote {
            // Remote template (e.g. @preview/jaconf)
            init_fn(&template_name, &paper_dir)?;
        } else {
            bail!(
                "Template '{}' not found and no remote initializer provided",
                template_name
            );
        }
    } else {
        // Default scaffolding
        fs::create_dir_all(&paper_dir)?;
        fs::create_dir(paper_dir.join("sections"))?;
        fs::create_dir(paper_dir.join("assets"))?;
        fs::create_dir(paper_dir.join("rules"))?;
        create_main_typ(&paper_dir)?;
        create_rules_readme(&paper_dir)?;
    }

    // Always create paper.toml if it wasn't provided by template
    if !paper_dir.join("paper.toml").exists() {
        create_paper_toml(&paper_dir, paper_id, title)?;
    }

    Ok(())
}

fn expand_local_template(src: &Path, dst: &Path, context: &TemplateContext) -> Result<()> {
    let engine = TemplateEngine::new();

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        // Check if file should be renamed (remove .tmp. from name)
        let (dst_file_name, is_template) = if file_name_str.contains(".tmp.") {
            (file_name_str.replace(".tmp.", "."), true)
        } else {
            (file_name_str.to_string(), false)
        };
        let dst_path = dst.join(dst_file_name);

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            expand_local_template(&src_path, &dst_path, context)?;
        } else if is_template {
            // Process as template
            let content = fs::read_to_string(&src_path)?;
            let expanded = engine.render(&content, context).map_err(|e| {
                anyhow::anyhow!("Template error in {}: {}", src_path.display(), e)
            })?;
            fs::write(dst_path, expanded)?;
        } else {
            // Check if it should be processed even without .tmp. (for backward compatibility or legacy title)
            if let Ok(content) = fs::read_to_string(&src_path) {
                // If it contains {{title}}, use simple replacement for legacy templates
                if content.contains("{{title}}") {
                    let title = context
                        .data()
                        .get("paper")
                        .and_then(|p| p.get("title"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("");
                    let expanded = content.replace("{{title}}", title);
                    fs::write(dst_path, expanded)?;
                } else {
                    fs::copy(src_path, dst_path)?;
                }
            } else {
                // binary files
                fs::copy(src_path, dst_path)?;
            }
        }
    }
    Ok(())
}

/// Create paper.toml with paper configuration
fn create_paper_toml(paper_dir: &Path, paper_id: &str, title: Option<String>) -> Result<()> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let title = title.unwrap_or_else(|| "New Paper".to_string());

    let content = format!(
        r#"[paper]
id = "{}"
title = "{}"
language = "en"
date = "{}"

# Uncomment to add authors
# [[paper.authors]]
# name = "Author Name"
# email = "author@example.com"
# affiliation = "University"

# Specified template to use
[template]
theme = "default"  # or "minimal", or custom template name

[build]
targets = ["pdf"]
# main_file = "main.typ"  # Default: main.typ
# root = "."              # Optional: root directory for Typst --root

[output]
name = "{}"
"#,
        paper_id, title, today, paper_id
    );

    fs::write(paper_dir.join("paper.toml"), content)?;
    Ok(())
}

/// Create main.typ with basic structure
fn create_main_typ(paper_dir: &Path) -> Result<()> {
    let content = r#"// Import generated metadata and header
#import "_generated/meta.typ": *
#import "_generated/header.typ": *

// Import generated references (uncomment when refs/ is populated)
// #import "_generated/refs.typ": bibliography

// Your content here
= Introduction

This is a new paper created with typstlab.

// Uncomment when you have references
// #bibliography("path/to/refs.bib")
"#;

    fs::write(paper_dir.join("main.typ"), content)?;
    Ok(())
}

/// Create rules/README.md with explanation
fn create_rules_readme(paper_dir: &Path) -> Result<()> {
    let content = r#"# Paper Rules Directory

This directory is for paper-specific MCP rules and documentation.

## Usage

Place Markdown files (`.md`) in this directory to provide context and instructions
specific to this paper. These rules can be accessed via MCP tools:

- `rules_list` - List all rule files in this paper
- `rules_get` - Retrieve full content of a rule file
- `rules_page` - Get paginated view of a rule file
- `rules_search` - Search across all rule files

## Examples

- `writing-style.md` - Style guide for this paper
- `terminology.md` - Domain-specific terms and definitions
- `outline.md` - Paper structure and section breakdown
- `references.md` - Key papers and citations
"#;

    fs::write(paper_dir.join("rules/README.md"), content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Project;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_project(root: &Path) -> Project {
        // Create minimal project structure
        fs::create_dir(root.join("papers")).unwrap();
        fs::create_dir(root.join("templates")).unwrap();
        fs::create_dir(root.join("refs")).unwrap();
        fs::create_dir(root.join("dist")).unwrap();
        fs::create_dir(root.join("rules")).unwrap();
        fs::create_dir(root.join(".typstlab")).unwrap();

        // Create typstlab.toml
        let config_content = r#"
[project]
name = "test-project"
init_date = "2026-01-15"

[typst]
version = "0.12.0"
"#;
        fs::write(root.join("typstlab.toml"), config_content).unwrap();

        // Load project
        Project::load(root.to_path_buf()).unwrap()
    }

    #[test]
    fn test_create_paper_success() {
        let temp = TempDir::new().unwrap();
        let project = create_test_project(temp.path());

        let result = create_paper::<fn(&str, &Path) -> Result<()>>(&project, "paper1", None, None, None);
        assert!(result.is_ok());

        let paper_dir = temp.path().join("papers/paper1");
        assert!(paper_dir.exists());
        assert!(paper_dir.join("paper.toml").exists());
        assert!(paper_dir.join("main.typ").exists());
        assert!(paper_dir.join("sections").is_dir());
        assert!(paper_dir.join("assets").is_dir());
        assert!(paper_dir.join("rules").is_dir());
        assert!(paper_dir.join("rules/README.md").exists());
    }

    #[test]
    fn test_create_paper_valid_toml() {
        let temp = TempDir::new().unwrap();
        let project = create_test_project(temp.path());

        create_paper::<fn(&str, &Path) -> Result<()>>(&project, "my-paper", None, None, None).unwrap();

        let toml_path = temp.path().join("papers/my-paper/paper.toml");
        let content = fs::read_to_string(toml_path).unwrap();

        assert!(content.contains("[paper]"));
        assert!(content.contains("id = \"my-paper\""));
        assert!(content.contains("[output]"));
        assert!(content.contains("name = \"my-paper\""));
        assert!(content.contains("[template]"));
    }

    #[test]
    fn test_create_paper_fails_if_exists() {
        let temp = TempDir::new().unwrap();
        let project = create_test_project(temp.path());

        // Create first time - should succeed
        create_paper::<fn(&str, &Path) -> Result<()>>(&project, "paper1", None, None, None).unwrap();

        // Create second time - should fail
        let result = create_paper::<fn(&str, &Path) -> Result<()>>(&project, "paper1", None, None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_create_paper_with_local_template() {
        let temp = TempDir::new().unwrap();
        let project = create_test_project(temp.path());

        // Create mock local template
        let template_dir = temp.path().join("templates").join("my_template");
        std::fs::create_dir_all(&template_dir).unwrap();
        std::fs::write(template_dir.join("main.typ"), "Hello {{title}}").unwrap();
        std::fs::write(template_dir.join("other.typ"), "Some {{title}} thing").unwrap();

        create_paper::<fn(&str, &Path) -> Result<()>>(
            &project,
            "paper_tmpl",
            Some("My Awesome Paper".to_string()),
            Some("my_template".to_string()),
            None,
        )
        .unwrap();

        let paper_dir = temp.path().join("papers/paper_tmpl");
        assert!(paper_dir.exists());

        // Check if template files were copied and variables substituted
        let main_content = std::fs::read_to_string(paper_dir.join("main.typ")).unwrap();
        assert_eq!(main_content, "Hello My Awesome Paper");

        let other_content = std::fs::read_to_string(paper_dir.join("other.typ")).unwrap();
        assert_eq!(other_content, "Some My Awesome Paper thing");
    }

    #[test]
    fn test_create_paper_with_remote_template() {
        let temp = TempDir::new().unwrap();
        let project = create_test_project(temp.path());

        let mut called = false;
        create_paper(
            &project,
            "paper_remote",
            None,
            Some("@preview/jaconf".to_string()),
            Some(|template: &str, path: &Path| {
                assert_eq!(template, "@preview/jaconf");
                std::fs::create_dir_all(path).unwrap();
                std::fs::write(path.join("main.typ"), "Remote Content").unwrap();
                called = true;
                Ok(())
            }),
        )
        .unwrap();

        assert!(called);
        let paper_dir = temp.path().join("papers/paper_remote");
        assert!(paper_dir.exists());
        let content = std::fs::read_to_string(paper_dir.join("main.typ")).unwrap();
        assert_eq!(content, "Remote Content");
    }
}
