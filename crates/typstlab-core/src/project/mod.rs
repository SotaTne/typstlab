//! Project detection and management

pub mod builtin_layouts;
pub mod create;
pub mod generate;
pub mod layout;

use crate::config::Config;
use crate::error::{Result, TypstlabError};
use crate::paper::Paper;
use std::path::{Path, PathBuf};

pub use create::{create_project, validate_name};
pub use generate::{generate_all_papers, generate_paper};
pub use layout::{resolve_layout, Layout};

/// Represents a typstlab project
#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    config: Config,
    papers: Vec<Paper>,
}

impl Project {
    /// Find project root by searching for typstlab.toml
    ///
    /// Traverses up the directory tree from `start`, looking for `typstlab.toml`.
    /// Returns `Ok(Some(Project))` if found, `Ok(None)` if not found.
    ///
    /// # Arguments
    ///
    /// * `start` - Starting directory path (will be canonicalized)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use typstlab_core::project::Project;
    /// use std::env;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let current_dir = env::current_dir()?;
    /// match Project::find_root(&current_dir)? {
    ///     Some(project) => println!("Found project at: {}", project.root.display()),
    ///     None => println!("Not in a typstlab project"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn find_root(start: &Path) -> Result<Option<Self>> {
        // Canonicalize to get absolute path and resolve symlinks
        let mut current = start.canonicalize()?;

        loop {
            // Check if typstlab.toml exists in current directory
            let config_path = current.join("typstlab.toml");
            if config_path.exists() && config_path.is_file() {
                return Ok(Some(Self::load(current)?));
            }

            // Move to parent directory
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => {
                    // Reached filesystem root without finding typstlab.toml
                    return Ok(None);
                }
            }
        }
    }

    /// Load a project from its root directory
    ///
    /// Reads typstlab.toml and discovers all papers in the papers/ directory.
    ///
    /// # Arguments
    ///
    /// * `root` - Project root directory containing typstlab.toml
    pub fn load(root: PathBuf) -> Result<Self> {
        let config = Config::from_file(root.join("typstlab.toml"))?;
        let papers = discover_papers(&root)?;
        Ok(Self {
            root,
            config,
            papers,
        })
    }

    /// Load project from current directory
    ///
    /// Searches for project root starting from current directory,
    /// then loads the project.
    pub fn from_current_dir() -> Result<Self> {
        let current = std::env::current_dir()?;
        match Self::find_root(&current)? {
            Some(project) => Ok(project),
            None => Err(TypstlabError::ProjectNotFound),
        }
    }

    /// Get project configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get all papers in the project
    pub fn papers(&self) -> &[Paper] {
        &self.papers
    }

    /// Find a paper by ID
    pub fn find_paper(&self, id: &str) -> Option<&Paper> {
        self.papers.iter().find(|p| p.id() == id)
    }
}

/// Discover all papers in the papers/ directory
///
/// Scans the papers/ directory and loads each valid paper.
/// Skips directories without valid paper.toml files.
fn discover_papers(root: &Path) -> Result<Vec<Paper>> {
    let papers_dir = root.join("papers");

    // If papers/ doesn't exist, return empty vec
    if !papers_dir.exists() {
        return Ok(Vec::new());
    }

    let mut papers = Vec::new();

    for entry in std::fs::read_dir(&papers_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip non-directories
        if !path.is_dir() {
            continue;
        }

        // Try to load paper, skip if invalid
        match Paper::load(path) {
            Ok(paper) => papers.push(paper),
            Err(_) => continue, // Skip invalid papers
        }
    }

    Ok(papers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use typstlab_testkit::temp_dir_in_workspace;

    fn create_test_project(root: &Path, project_name: &str, papers: Vec<(&str, &str)>) {
        // Create typstlab.toml
        let config = format!(
            r#"
[project]
name = "{}"
init_date = "2026-01-14"

[typst]
version = "0.12.0"
"#,
            project_name
        );
        std::fs::write(root.join("typstlab.toml"), config).unwrap();

        // Create papers/ directory
        let papers_dir = root.join("papers");
        std::fs::create_dir(&papers_dir).unwrap();

        // Create each paper
        for (id, title) in papers {
            let paper_dir = papers_dir.join(id);
            std::fs::create_dir(&paper_dir).unwrap();

            let paper_config = format!(
                r#"
[paper]
id = "{}"
title = "{}"
language = "en"
date = "2026-01-14"

[output]
name = "{}"
"#,
                id, title, id
            );
            std::fs::write(paper_dir.join("paper.toml"), paper_config).unwrap();
        }
    }

    #[test]
    fn test_project_load_with_config() {
        let temp = temp_dir_in_workspace();
        create_test_project(temp.path(), "test-project", vec![]);

        let project = Project::load(temp.path().to_path_buf()).unwrap();
        assert_eq!(project.config().project.name, "test-project");
        assert_eq!(project.config().typst.version, "0.12.0");
    }

    #[test]
    fn test_project_load_discovers_papers() {
        let temp = temp_dir_in_workspace();
        create_test_project(
            temp.path(),
            "test-project",
            vec![("paper1", "Paper One"), ("paper2", "Paper Two")],
        );

        let project = Project::load(temp.path().to_path_buf()).unwrap();
        assert_eq!(project.papers().len(), 2);

        let ids: Vec<&str> = project.papers().iter().map(|p| p.id()).collect();
        assert!(ids.contains(&"paper1"));
        assert!(ids.contains(&"paper2"));
    }

    #[test]
    fn test_project_load_empty_papers_dir() {
        let temp = temp_dir_in_workspace();
        create_test_project(temp.path(), "test-project", vec![]);

        let project = Project::load(temp.path().to_path_buf()).unwrap();
        assert_eq!(project.papers().len(), 0);
    }

    #[test]
    fn test_project_find_paper_by_id() {
        let temp = temp_dir_in_workspace();
        create_test_project(
            temp.path(),
            "test-project",
            vec![("paper1", "Paper One"), ("paper2", "Paper Two")],
        );

        let project = Project::load(temp.path().to_path_buf()).unwrap();

        let paper = project.find_paper("paper1");
        assert!(paper.is_some());
        assert_eq!(paper.unwrap().config().paper.title, "Paper One");

        let paper = project.find_paper("nonexistent");
        assert!(paper.is_none());
    }

    // Note: test_project_from_current_dir() was removed because std::env::set_current_dir()
    // causes race conditions in parallel test execution. The functionality is covered by
    // test_find_root tests in integration tests.

    #[test]
    fn test_discover_papers_skips_invalid_toml() {
        let temp = temp_dir_in_workspace();
        let papers_dir = temp.path().join("papers");
        std::fs::create_dir_all(&papers_dir).unwrap();

        // Create valid paper
        let paper1_dir = papers_dir.join("paper1");
        std::fs::create_dir(&paper1_dir).unwrap();
        std::fs::write(
            paper1_dir.join("paper.toml"),
            r#"
[paper]
id = "paper1"
title = "Valid Paper"
language = "en"
date = "2026-01-14"

[output]
name = "paper1"
"#,
        )
        .unwrap();

        // Create invalid paper (missing required fields)
        let paper2_dir = papers_dir.join("paper2");
        std::fs::create_dir(&paper2_dir).unwrap();
        std::fs::write(
            paper2_dir.join("paper.toml"),
            r#"
[paper]
id = "paper2"
"#,
        )
        .unwrap();

        // Create directory without paper.toml
        let paper3_dir = papers_dir.join("paper3");
        std::fs::create_dir(&paper3_dir).unwrap();

        let papers = discover_papers(temp.path()).unwrap();
        assert_eq!(papers.len(), 1);
        assert_eq!(papers[0].id(), "paper1");
    }
}
