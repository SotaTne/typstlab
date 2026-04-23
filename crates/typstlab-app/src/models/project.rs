use crate::models::build_artifact_scope::BuildArtifactScope;
use crate::models::paper_scope::PaperScope;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use typstlab_proto::{Creatable, Entity, Loadable};

#[derive(Error, Debug)]
pub enum ProjectError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("Not initialized")]
    NotInitialized,
}

// ... (ProjectConfig 等の定義は変更なし)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectConfig {
    pub project: ProjectInfo,
    #[serde(default)]
    pub typst: TypstInfo,
    #[serde(default)]
    pub structure: StructureConfig,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            project: ProjectInfo::default(),
            typst: TypstInfo::default(),
            structure: StructureConfig::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectInfo {
    pub name: String,
    #[serde(default = "default_init_date")]
    pub init_date: String,
}

impl Default for ProjectInfo {
    fn default() -> Self {
        Self {
            name: default_project_name(),
            init_date: default_init_date(),
        }
    }
}

fn default_project_name() -> String {
    "unnamed-project".to_string()
}
fn default_init_date() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TypstInfo {
    #[serde(default = "default_typst_version")]
    pub version: String,
}

impl Default for TypstInfo {
    fn default() -> Self {
        Self {
            version: default_typst_version(),
        }
    }
}

fn default_typst_version() -> String {
    "0.14.2".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StructureConfig {
    #[serde(default = "default_papers_dir")]
    pub papers_dir: PathBuf,
    #[serde(default = "default_dist_dir")]
    pub dist_dir: PathBuf,
}

impl Default for StructureConfig {
    fn default() -> Self {
        Self {
            papers_dir: default_papers_dir(),
            dist_dir: default_dist_dir(),
        }
    }
}

fn default_papers_dir() -> PathBuf {
    PathBuf::from("papers")
}
fn default_dist_dir() -> PathBuf {
    PathBuf::from("dist")
}

pub struct Project {
    pub root: PathBuf,
    pub config: Option<ProjectConfig>,
}

impl Project {
    pub fn new(root: PathBuf) -> Self {
        Self { root, config: None }
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("typstlab.toml")
    }

    pub fn papers_scope(&self) -> PaperScope {
        let papers_dir = self
            .config
            .as_ref()
            .map(|c| c.structure.papers_dir.clone())
            .unwrap_or_else(default_papers_dir);
        PaperScope::new(self.root.clone(), papers_dir)
    }

    pub fn build_artifact_scope(&self) -> BuildArtifactScope {
        let dist_dir = self
            .config
            .as_ref()
            .map(|c| c.structure.dist_dir.clone())
            .unwrap_or_else(default_dist_dir);
        BuildArtifactScope::new(self.root.clone(), dist_dir)
    }
}

impl Entity for Project {
    fn path(&self) -> PathBuf {
        self.root.clone()
    }
}

impl Loadable for Project {
    type Config = ProjectConfig;
    type Error = ProjectError;

    fn load_from_disk(&self) -> Result<Self::Config, Self::Error> {
        let content = std::fs::read_to_string(self.config_path())?;
        let config: ProjectConfig = toml::from_str(&content)?;
        Ok(config)
    }

    fn apply_config(&mut self, config: Self::Config) {
        self.config = Some(config);
    }
}

pub struct ProjectCreationArgs {
    pub name: String,
}

impl Creatable for Project {
    type Args = ProjectCreationArgs;

    fn initialize(&mut self, args: Self::Args) {
        self.config = Some(ProjectConfig {
            project: ProjectInfo {
                name: args.name,
                init_date: default_init_date(),
            },
            typst: TypstInfo::default(),
            structure: StructureConfig::default(),
        });
    }

    fn persist(&self) -> Result<(), String> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| "Project not initialized. Call initialize() first.".to_string())?;

        let toml_content = toml::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        if !self.root.exists() {
            std::fs::create_dir_all(&self.root)
                .map_err(|e| format!("Failed to create project directory: {}", e))?;
        }

        std::fs::write(self.config_path(), toml_content)
            .map_err(|e| format!("Failed to write typstlab.toml: {}", e))?;

        let _ = std::fs::create_dir_all(self.papers_scope().path());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Project, ProjectConfig, ProjectInfo, StructureConfig, TypstInfo};
    use std::path::PathBuf;
    use typstlab_proto::Entity;

    #[test]
    fn test_papers_scope_uses_pathbuf_internally() {
        let root = PathBuf::from("/project-root");
        let mut project = Project::new(root.clone());
        project.config = Some(ProjectConfig {
            project: ProjectInfo {
                name: "demo".to_string(),
                init_date: "2026-04-23".to_string(),
            },
            typst: TypstInfo {
                version: "0.14.2".to_string(),
            },
            structure: StructureConfig {
                papers_dir: PathBuf::from("content").join("papers"),
                dist_dir: PathBuf::from("out").join("dist"),
            },
        });

        let scope = project.papers_scope();

        assert_eq!(scope.relative_path, PathBuf::from("content").join("papers"));
        assert_eq!(scope.path(), root.join("content").join("papers"));
    }

    #[test]
    fn test_build_artifact_scope_uses_pathbuf_internally() {
        let root = PathBuf::from("/project-root");
        let mut project = Project::new(root.clone());
        project.config = Some(ProjectConfig {
            project: ProjectInfo {
                name: "demo".to_string(),
                init_date: "2026-04-23".to_string(),
            },
            typst: TypstInfo {
                version: "0.14.2".to_string(),
            },
            structure: StructureConfig {
                papers_dir: PathBuf::from("content").join("papers"),
                dist_dir: PathBuf::from("out").join("dist"),
            },
        });

        let scope = project.build_artifact_scope();

        assert_eq!(scope.relative_path, PathBuf::from("out").join("dist"));
        assert_eq!(scope.path(), root.join("out").join("dist"));
    }

    #[test]
    fn test_config_deserializes_structure_paths_as_pathbuf() {
        let config: ProjectConfig = toml::from_str(
            r#"
                [project]
                name = "demo"
                init_date = "2026-04-23"

                [typst]
                version = "0.14.2"

                [structure]
                papers_dir = "content/papers"
                dist_dir = "out/dist"
            "#,
        )
        .unwrap();

        assert_eq!(
            config.structure.papers_dir,
            PathBuf::from("content").join("papers")
        );
        assert_eq!(config.structure.dist_dir, PathBuf::from("out").join("dist"));
    }
}
