use crate::models::build_artifact_scope::BuildArtifactScope;
use crate::models::paper_scope::PaperScope;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use typstlab_proto::{Creatable, Entity, Loadable, Loaded, PROJECT_SETTING_FILE};

#[derive(Error, Debug)]
pub enum ProjectError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("Not initialized")]
    NotInitialized,
}

// ... (ProjectConfig 等の定義は変更なし)
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ProjectConfig {
    pub project: ProjectInfo,
    #[serde(default)]
    pub typst: TypstInfo,
    #[serde(default)]
    pub structure: StructureConfig,
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
    #[serde(default = "default_templates_dir")]
    pub templates_dir: PathBuf,
}

impl Default for StructureConfig {
    fn default() -> Self {
        Self {
            papers_dir: default_papers_dir(),
            dist_dir: default_dist_dir(),
            templates_dir: default_templates_dir(),
        }
    }
}

fn default_papers_dir() -> PathBuf {
    PathBuf::from("papers")
}
fn default_dist_dir() -> PathBuf {
    PathBuf::from("dist")
}
fn default_templates_dir() -> PathBuf {
    PathBuf::from("templates")
}

pub struct Project {
    pub root: PathBuf,
}

typstlab_proto::impl_entity! {
    Project {
        fn path(&self) -> PathBuf {
            self.root.clone()
        }
    }
}

impl Project {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join(PROJECT_SETTING_FILE)
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
}

pub struct ProjectCreationArgs {
    pub name: String,
}

impl Creatable for Project {
    type Args = ProjectCreationArgs;
    type Config = ProjectConfig;
    type Error = ProjectError;

    fn initialize(self, args: Self::Args) -> Result<Loaded<Self, Self::Config>, Self::Error> {
        Ok(Loaded {
            actual: self,
            config: ProjectConfig {
                project: ProjectInfo {
                    name: args.name,
                    init_date: default_init_date(),
                },
                typst: TypstInfo::default(),
                structure: StructureConfig::default(),
            },
        })
    }

    fn persist(loaded: &Loaded<Self, Self::Config>) -> Result<(), Self::Error> {
        let toml_content = toml::to_string_pretty(&loaded.config)?;

        if !loaded.actual.root.exists() {
            std::fs::create_dir_all(&loaded.actual.root)?;
        }

        std::fs::write(loaded.actual.config_path(), toml_content)?;

        std::fs::create_dir_all(ProjectHandle::papers_scope(loaded).path())?;

        Ok(())
    }
}

pub trait ProjectHandle {
    fn papers_scope(&self) -> PaperScope;
    fn templates_scope(&self) -> crate::models::template_scope::TemplateScope;
    fn build_artifact_scope(&self) -> BuildArtifactScope;
    fn name(&self) -> &str;
    fn typst_version(&self) -> &str;
}

impl ProjectHandle for Loaded<Project, ProjectConfig> {
    fn papers_scope(&self) -> PaperScope {
        PaperScope::new(
            self.actual.root.clone(),
            self.config.structure.papers_dir.clone(),
        )
    }

    fn templates_scope(&self) -> crate::models::template_scope::TemplateScope {
        crate::models::template_scope::TemplateScope::new(
            self.actual.root.clone(),
            self.config.structure.templates_dir.clone(),
        )
    }

    fn build_artifact_scope(&self) -> BuildArtifactScope {
        BuildArtifactScope::new(
            self.actual.root.clone(),
            self.config.structure.dist_dir.clone(),
        )
    }

    fn name(&self) -> &str {
        &self.config.project.name
    }

    fn typst_version(&self) -> &str {
        &self.config.typst.version
    }
}

#[cfg(test)]
mod tests {
    use super::{Project, ProjectConfig, ProjectHandle, ProjectInfo, StructureConfig, TypstInfo};
    use std::path::PathBuf;
    use typstlab_proto::{Entity, Loaded};

    fn loaded_project(root: &str) -> Loaded<Project, ProjectConfig> {
        Loaded {
            actual: Project::new(PathBuf::from(root)),
            config: ProjectConfig {
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
                    templates_dir: PathBuf::from("assets").join("templates"),
                },
            },
        }
    }

    #[test]
    fn test_papers_scope_uses_pathbuf_internally() {
        let project = loaded_project("/project-root");
        let scope = project.papers_scope();

        assert_eq!(scope.relative_path, PathBuf::from("content").join("papers"));
        assert_eq!(
            scope.path(),
            PathBuf::from("/project-root")
                .join("content")
                .join("papers")
        );
    }

    #[test]
    fn test_build_artifact_scope_uses_pathbuf_internally() {
        let project = loaded_project("/project-root");
        let scope = project.build_artifact_scope();

        assert_eq!(scope.relative_path, PathBuf::from("out").join("dist"));
        assert_eq!(
            scope.path(),
            PathBuf::from("/project-root").join("out").join("dist")
        );
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
