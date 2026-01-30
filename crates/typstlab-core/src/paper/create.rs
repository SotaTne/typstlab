//! Paper scaffold creation

use crate::project::{validate_name, Project};
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
/// Note: _generated/ directory is NOT created by this function.
/// The caller should reload the project and call generate_paper() afterwards.
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
pub fn create_paper(project: &Project, paper_id: &str, title: Option<String>) -> Result<()> {
    // Validate paper ID for security
    validate_name(paper_id)?;

    let papers_dir = project.root.join("papers");
    let paper_dir = papers_dir.join(paper_id);

    // Check if paper already exists
    if paper_dir.exists() {
        bail!("Paper '{}' already exists", paper_id);
    }

    // Create paper directory
    fs::create_dir(&paper_dir)?;

    // Create subdirectories
    fs::create_dir(paper_dir.join("sections"))?;
    fs::create_dir(paper_dir.join("assets"))?;
    fs::create_dir(paper_dir.join("rules"))?;

    // Create paper.toml
    create_paper_toml(&paper_dir, paper_id, title)?;

    // Create main.typ
    create_main_typ(&paper_dir)?;

    // Create rules/README.md
    create_rules_readme(&paper_dir)?;

    // Note: _generated/ directory will be created by caller after reloading project
    // This is necessary because generate_paper() needs the paper to be in project.papers

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

# Uncomment to specify custom layout
# [layout]
# theme = "default"  # or "minimal", or custom layout name

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
        fs::create_dir(root.join("layouts")).unwrap();
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

        let result = create_paper(&project, "paper1", None);
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

        create_paper(&project, "my-paper", None).unwrap();

        let toml_path = temp.path().join("papers/my-paper/paper.toml");
        let content = fs::read_to_string(toml_path).unwrap();

        assert!(content.contains("[paper]"));
        assert!(content.contains("id = \"my-paper\""));
        assert!(content.contains("[output]"));
        assert!(content.contains("name = \"my-paper\""));
    }

    #[test]
    fn test_create_paper_valid_main_typ() {
        let temp = TempDir::new().unwrap();
        let project = create_test_project(temp.path());

        create_paper(&project, "paper1", None).unwrap();

        let main_typ_path = temp.path().join("papers/paper1/main.typ");
        let content = fs::read_to_string(main_typ_path).unwrap();

        assert!(content.contains("#import"));
        assert!(content.contains("_generated/meta.typ"));
        assert!(!content.is_empty());
    }

    #[test]
    fn test_create_paper_fails_if_exists() {
        let temp = TempDir::new().unwrap();
        let project = create_test_project(temp.path());

        // Create first time - should succeed
        create_paper(&project, "paper1", None).unwrap();

        // Create second time - should fail
        let result = create_paper(&project, "paper1", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_create_paper_without_generated() {
        let temp = TempDir::new().unwrap();
        let project = create_test_project(temp.path());

        create_paper(&project, "paper1", None).unwrap();

        // _generated/ is not created by create_paper
        // It will be created by caller after reloading project
        let generated_dir = temp.path().join("papers/paper1/_generated");
        assert!(!generated_dir.exists());
    }
}
