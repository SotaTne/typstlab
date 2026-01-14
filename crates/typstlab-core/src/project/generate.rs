//! Paper generation logic - creates _generated/ directory with rendered templates

use crate::error::{Result, TypstlabError};
use crate::project::{resolve_layout, Project};
use crate::template::{TemplateContext, TemplateEngine};
use std::fs;
use tempfile::TempDir;
use toml::Value;

/// Generate _generated/ directory for a single paper
///
/// # Steps
/// 1. Load paper config → create TemplateContext
/// 2. Resolve layout (user layout or builtin)
/// 3. Render templates:
///    - `meta.tmp.typ` → `_generated/meta.typ`
///    - `header.typ` → `_generated/header.typ` (copy if static)
///    - `refs.tmp.typ` → `_generated/refs.typ`
/// 4. Write to temp directory → atomic rename
///
/// # Arguments
/// * `project` - Project instance
/// * `paper_id` - ID of the paper to generate
pub fn generate_paper(project: &Project, paper_id: &str) -> Result<()> {
    // Find paper by ID
    let paper = project
        .find_paper(paper_id)
        .ok_or_else(|| TypstlabError::PaperNotFound(paper_id.to_string()))?;

    // Convert PaperConfig to toml::Value for template context
    let config_toml =
        toml::to_string(paper.config()).map_err(|e| TypstlabError::Generic(e.to_string()))?;
    let mut data: Value =
        toml::from_str(&config_toml).map_err(|e| TypstlabError::Generic(e.to_string()))?;

    // Ensure refs.sets exists (empty array if refs is None)
    if let Value::Table(ref mut table) = data {
        if !table.contains_key("refs") {
            let mut refs_table = toml::map::Map::new();
            refs_table.insert("sets".to_string(), Value::Array(vec![]));
            table.insert("refs".to_string(), Value::Table(refs_table));
        }
    }

    let context = TemplateContext::new(data);

    // Resolve layout (user layout or builtin)
    let layout_theme = &paper.config().layout.theme;
    let layout = resolve_layout(&project.root, layout_theme)?;

    // Create temp directory for atomic generation
    let temp_dir = TempDir::new_in(&project.root)?;
    let temp_generated = temp_dir.path().join("_generated");
    fs::create_dir(&temp_generated)?;

    // Render and write templates
    let engine = TemplateEngine::new();

    // meta.tmp.typ → _generated/meta.typ
    if let Some(meta_template) = &layout.meta_template {
        let rendered = engine.render(meta_template, &context)?;
        fs::write(temp_generated.join("meta.typ"), rendered)?;
    }

    // header.typ → _generated/header.typ (copy if static)
    if let Some(header_static) = &layout.header_static {
        fs::write(temp_generated.join("header.typ"), header_static)?;
    }

    // refs.tmp.typ → _generated/refs.typ
    if let Some(refs_template) = &layout.refs_template {
        let rendered = engine.render(refs_template, &context)?;
        fs::write(temp_generated.join("refs.typ"), rendered)?;
    }

    // Atomic rename: temp → paper/_generated/
    let target_generated = paper.root().join("_generated");

    // Remove old _generated/ if exists
    if target_generated.exists() {
        fs::remove_dir_all(&target_generated)?;
    }

    // Rename temp to target (atomic on same filesystem)
    fs::rename(&temp_generated, &target_generated)?;

    Ok(())
}

/// Generate _generated/ directory for all papers
///
/// Returns list of successfully generated paper IDs.
/// Failures are logged but don't stop processing of other papers.
///
/// # Arguments
/// * `project` - Project instance
pub fn generate_all_papers(project: &Project) -> Result<Vec<String>> {
    let mut generated = Vec::new();

    for paper in project.papers() {
        let paper_id = paper.id();
        match generate_paper(project, paper_id) {
            Ok(()) => {
                generated.push(paper_id.to_string());
            }
            Err(e) => {
                eprintln!("Warning: Failed to generate {}: {}", paper_id, e);
                // Continue processing other papers
            }
        }
    }

    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use typstlab_testkit::temp_dir_in_workspace;

    fn create_test_project(root: &Path) {
        // Create typstlab.toml
        let config = r#"
[project]
name = "test-project"
init_date = "2026-01-15"

[typst]
version = "0.12.0"
"#;
        fs::write(root.join("typstlab.toml"), config).unwrap();

        // Create papers/ directory
        let papers_dir = root.join("papers");
        fs::create_dir(&papers_dir).unwrap();

        // Create paper1 with authors
        let paper1_dir = papers_dir.join("paper1");
        fs::create_dir_all(&paper1_dir).unwrap();
        let paper1_config = r#"
[paper]
id = "paper1"
title = "Test Paper One"
language = "en"
date = "2026-01-15"

[[paper.authors]]
name = "Alice"
email = "alice@example.com"
affiliation = "University A"

[[paper.authors]]
name = "Bob"
email = "bob@example.com"
affiliation = "University B"

[output]
name = "paper1"
"#;
        fs::write(paper1_dir.join("paper.toml"), paper1_config).unwrap();
        fs::write(paper1_dir.join("main.typ"), "// Main content").unwrap();

        // Create paper2 without authors (minimal)
        let paper2_dir = papers_dir.join("paper2");
        fs::create_dir_all(&paper2_dir).unwrap();
        let paper2_config = r#"
[paper]
id = "paper2"
title = "Test Paper Two"
language = "en"
date = "2026-01-15"

[layout]
theme = "minimal"

[output]
name = "paper2"
"#;
        fs::write(paper2_dir.join("paper.toml"), paper2_config).unwrap();
        fs::write(paper2_dir.join("main.typ"), "// Main content").unwrap();
    }

    #[test]
    fn test_generate_with_default_layout() {
        let temp = temp_dir_in_workspace();
        create_test_project(temp.path());

        let project = Project::load(temp.path().to_path_buf()).unwrap();
        generate_paper(&project, "paper1").unwrap();

        // Verify _generated/ directory created
        let generated_dir = temp.path().join("papers/paper1/_generated");
        assert!(generated_dir.exists());
        assert!(generated_dir.is_dir());

        // Verify meta.typ exists and contains rendered content
        let meta_path = generated_dir.join("meta.typ");
        assert!(meta_path.exists());
        let meta_content = fs::read_to_string(&meta_path).unwrap();
        assert!(meta_content.contains("Test Paper One"));
        assert!(meta_content.contains("Alice"));
        assert!(meta_content.contains("Bob"));

        // Verify header.typ exists (default layout has header)
        let header_path = generated_dir.join("header.typ");
        assert!(header_path.exists());

        // Verify refs.typ exists
        let refs_path = generated_dir.join("refs.typ");
        assert!(refs_path.exists());
    }

    #[test]
    fn test_generate_with_minimal_layout() {
        let temp = temp_dir_in_workspace();
        create_test_project(temp.path());

        let project = Project::load(temp.path().to_path_buf()).unwrap();
        generate_paper(&project, "paper2").unwrap();

        let generated_dir = temp.path().join("papers/paper2/_generated");
        assert!(generated_dir.exists());

        // Verify meta.typ exists
        let meta_path = generated_dir.join("meta.typ");
        assert!(meta_path.exists());
        let meta_content = fs::read_to_string(&meta_path).unwrap();
        assert!(meta_content.contains("Test Paper Two"));

        // Verify header.typ does NOT exist (minimal layout has no header)
        let header_path = generated_dir.join("header.typ");
        assert!(!header_path.exists());

        // Verify refs.typ exists
        let refs_path = generated_dir.join("refs.typ");
        assert!(refs_path.exists());
    }

    #[test]
    fn test_generate_with_custom_layout() {
        let temp = temp_dir_in_workspace();
        create_test_project(temp.path());

        // Create custom layout
        let layouts_dir = temp.path().join("layouts/custom");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::write(
            layouts_dir.join("meta.tmp.typ"),
            "= Custom: {{ paper.title }}",
        )
        .unwrap();

        // Create paper3 with custom layout
        let paper3_dir = temp.path().join("papers/paper3");
        fs::create_dir_all(&paper3_dir).unwrap();
        let paper3_config = r#"
[paper]
id = "paper3"
title = "Custom Layout Paper"
language = "en"
date = "2026-01-15"

[layout]
theme = "custom"

[output]
name = "paper3"
"#;
        fs::write(paper3_dir.join("paper.toml"), paper3_config).unwrap();

        let project = Project::load(temp.path().to_path_buf()).unwrap();
        generate_paper(&project, "paper3").unwrap();

        let meta_path = temp.path().join("papers/paper3/_generated/meta.typ");
        let meta_content = fs::read_to_string(&meta_path).unwrap();
        assert_eq!(meta_content, "= Custom: Custom Layout Paper");
    }

    #[test]
    fn test_generate_authors_list() {
        let temp = temp_dir_in_workspace();
        create_test_project(temp.path());

        let project = Project::load(temp.path().to_path_buf()).unwrap();
        generate_paper(&project, "paper1").unwrap();

        let meta_path = temp.path().join("papers/paper1/_generated/meta.typ");
        let meta_content = fs::read_to_string(&meta_path).unwrap();

        // Verify authors rendered with {{each}} loop
        assert!(meta_content.contains("Alice"));
        assert!(meta_content.contains("alice@example.com"));
        assert!(meta_content.contains("Bob"));
        assert!(meta_content.contains("bob@example.com"));
    }

    #[test]
    fn test_generate_atomic_operation() {
        let temp = temp_dir_in_workspace();
        create_test_project(temp.path());

        let generated_dir = temp.path().join("papers/paper1/_generated");

        // Create existing _generated/ with old content
        fs::create_dir_all(&generated_dir).unwrap();
        fs::write(generated_dir.join("meta.typ"), "OLD CONTENT").unwrap();

        let project = Project::load(temp.path().to_path_buf()).unwrap();
        generate_paper(&project, "paper1").unwrap();

        // Verify old content replaced atomically
        let meta_content = fs::read_to_string(generated_dir.join("meta.typ")).unwrap();
        assert!(meta_content.contains("Test Paper One"));
        assert!(!meta_content.contains("OLD CONTENT"));
    }

    #[test]
    fn test_generate_all_papers() {
        let temp = temp_dir_in_workspace();
        create_test_project(temp.path());

        let project = Project::load(temp.path().to_path_buf()).unwrap();
        let generated = generate_all_papers(&project).unwrap();

        assert_eq!(generated.len(), 2);
        assert!(generated.contains(&"paper1".to_string()));
        assert!(generated.contains(&"paper2".to_string()));

        // Verify both papers generated
        assert!(temp
            .path()
            .join("papers/paper1/_generated/meta.typ")
            .exists());
        assert!(temp
            .path()
            .join("papers/paper2/_generated/meta.typ")
            .exists());
    }

    #[test]
    fn test_generate_paper_not_found() {
        let temp = temp_dir_in_workspace();
        create_test_project(temp.path());

        let project = Project::load(temp.path().to_path_buf()).unwrap();
        let result = generate_paper(&project, "nonexistent");

        assert!(result.is_err());
        match result {
            Err(TypstlabError::PaperNotFound(id)) => assert_eq!(id, "nonexistent"),
            _ => panic!("Expected PaperNotFound error"),
        }
    }

    #[test]
    fn test_generate_layout_not_found() {
        let temp = temp_dir_in_workspace();
        let papers_dir = temp.path().join("papers/paper_bad");
        fs::create_dir_all(&papers_dir).unwrap();

        // Create typstlab.toml
        fs::write(
            temp.path().join("typstlab.toml"),
            r#"
[project]
name = "test"
init_date = "2026-01-15"

[typst]
version = "0.12.0"
"#,
        )
        .unwrap();

        // Create paper with nonexistent layout
        let paper_config = r#"
[paper]
id = "paper_bad"
title = "Bad Layout"
language = "en"
date = "2026-01-15"

[layout]
theme = "nonexistent"

[output]
name = "paper_bad"
"#;
        fs::write(papers_dir.join("paper.toml"), paper_config).unwrap();

        let project = Project::load(temp.path().to_path_buf()).unwrap();
        let result = generate_paper(&project, "paper_bad");

        assert!(result.is_err());
        match result {
            Err(TypstlabError::LayoutNotFound(name)) => assert_eq!(name, "nonexistent"),
            _ => panic!("Expected LayoutNotFound error"),
        }
    }
}
