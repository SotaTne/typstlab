use crate::actions::toolchain_resolve::ToolChain;
use crate::models::project::ProjectHandle;
use crate::models::template_scope::TemplateScope;
use crate::models::{CollectionError, Paper, PaperScope, Project, ProjectConfig, Template};
use serde::Serialize;
use std::path::{Path, PathBuf};
use thiserror::Error;
use typstlab_proto::{Action, Collection, Entity, Loaded};

#[derive(Serialize, Debug, Clone)]
pub struct StatusOutput {
    pub project: ProjectStatus,
    pub toolchain: ToolchainStatus,
    pub docs: Option<DocsStatus>,
    pub papers: ScopeStatus,
    pub templates: ScopeStatus,
    pub dist: DirectoryStatus,
}

#[derive(Serialize, Debug, Clone)]
pub struct ProjectStatus {
    pub name: String,
    pub root_path: PathBuf,
}

#[derive(Serialize, Debug, Clone)]
pub struct ToolchainStatus {
    pub typst: TypstStatus,
}

#[derive(Serialize, Debug, Clone)]
pub struct TypstStatus {
    pub version: String,
    pub path_in_store: PathBuf,
}

#[derive(Serialize, Debug, Clone)]
pub struct DocsStatus {
    pub path_in_store: PathBuf,
}

#[derive(Serialize, Debug, Clone)]
pub struct ScopeStatus {
    pub root: PathBuf,
    pub items: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct DirectoryStatus {
    pub root: PathBuf,
}

pub struct StatusAction {
    pub loaded_project: Loaded<Project, ProjectConfig>,
    pub toolchain: ToolChain,
}

impl StatusAction {
    pub fn new(loaded_project: Loaded<Project, ProjectConfig>, toolchain: ToolChain) -> Self {
        Self {
            loaded_project,
            toolchain,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum StatusWarning {
    PapersDirNotFound(PathBuf),
    TemplatesDirNotFound(PathBuf),
    DistDirNotFound(PathBuf),
}

#[derive(Debug, Error)]
pub enum StatusError {
    #[error("failed to read papers directory: {0}")]
    PapersReadFailed(#[source] CollectionError),
    #[error("failed to read templates directory: {0}")]
    TemplatesReadFailed(#[source] CollectionError),
    #[error("failed to inspect directory '{path}': {source}")]
    DirectoryInspectFailed {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl Action for StatusAction {
    type Output = StatusOutput;
    type Event = ();
    type Warning = StatusWarning;
    type Error = StatusError;

    fn run(
        self,
        _monitor: &mut dyn FnMut(typstlab_proto::AppEvent<Self::Event>),
        warning: &mut dyn FnMut(Self::Warning),
    ) -> Result<Self::Output, Vec<Self::Error>> {
        let papers_scope = self.loaded_project.papers_scope();
        let templates_scope = self.loaded_project.templates_scope();
        let dist_scope = self.loaded_project.build_artifact_scope();

        let papers = list_papers(&papers_scope, warning)?;
        let templates = list_templates(&templates_scope, warning)?;
        warn_if_missing(&dist_scope.path(), StatusWarning::DistDirNotFound, warning)?;

        Ok(StatusOutput {
            project: ProjectStatus {
                name: self.loaded_project.name().to_string(),
                root_path: self.loaded_project.actual.root.clone(),
            },
            toolchain: ToolchainStatus {
                typst: TypstStatus {
                    version: self.toolchain.typst.version.clone(),
                    path_in_store: self.toolchain.typst.path(),
                },
            },
            docs: self.toolchain.typst_docs.map(|docs| DocsStatus {
                path_in_store: docs.path(),
            }),
            papers: ScopeStatus {
                root: papers_scope.path(),
                items: papers.into_iter().map(|p| p.id).collect(),
            },
            templates: ScopeStatus {
                root: templates_scope.path(),
                items: templates.into_iter().map(|t| t.id).collect(),
            },
            dist: DirectoryStatus {
                root: dist_scope.path(),
            },
        })
    }
}

fn list_papers(
    scope: &PaperScope,
    warning: &mut dyn FnMut(StatusWarning),
) -> Result<Vec<Paper>, Vec<StatusError>> {
    match scope.list() {
        Ok(papers) => Ok(papers),
        Err(CollectionError::NotFound(path)) => {
            warning(StatusWarning::PapersDirNotFound(path));
            Ok(Vec::new())
        }
        Err(error) => Err(vec![StatusError::PapersReadFailed(error)]),
    }
}

fn list_templates(
    scope: &TemplateScope,
    warning: &mut dyn FnMut(StatusWarning),
) -> Result<Vec<Template>, Vec<StatusError>> {
    let root = scope.path();
    if !path_exists(&root)? {
        warning(StatusWarning::TemplatesDirNotFound(root));
        return Ok(Vec::new());
    }

    scope
        .list()
        .map_err(|error| vec![StatusError::TemplatesReadFailed(error)])
}

fn warn_if_missing(
    path: &Path,
    warning_kind: impl FnOnce(PathBuf) -> StatusWarning,
    warning: &mut dyn FnMut(StatusWarning),
) -> Result<(), Vec<StatusError>> {
    if !path_exists(path)? {
        warning(warning_kind(path.to_path_buf()));
    }
    Ok(())
}

fn path_exists(path: &Path) -> Result<bool, Vec<StatusError>> {
    path.try_exists().map_err(|source| {
        vec![StatusError::DirectoryInspectFailed {
            path: path.to_path_buf(),
            source,
        }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::{ProjectInfo, StructureConfig};
    use crate::models::{Docs, Project, ProjectConfig, ProjectToolChain, Typst};
    use tempfile::TempDir;

    fn dummy_loaded_project(root: PathBuf) -> Loaded<Project, ProjectConfig> {
        Loaded {
            actual: Project::new(root),
            config: ProjectConfig {
                project: ProjectInfo {
                    name: "demo".to_string(),
                    init_date: "2026-04-23".to_string(),
                },
                toolchain: ProjectToolChain::default(),
                structure: StructureConfig::default(),
            },
        }
    }

    fn dummy_toolchain() -> ToolChain {
        ToolChain {
            typst: Typst::new("0.14.2".to_string(), PathBuf::from("/bin/typst")),
            typst_docs: Some(Docs::new(PathBuf::from("/docs"))),
            typstyle: None,
        }
    }

    #[test]
    fn test_status_action_warns_on_missing_papers_dir() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path().to_path_buf();
        std::fs::create_dir_all(project_root.join("templates")).unwrap();
        std::fs::create_dir_all(project_root.join("dist")).unwrap();

        // do NOT create papers directory

        let loaded = dummy_loaded_project(project_root.clone());
        let action = StatusAction::new(loaded, dummy_toolchain());

        let mut warnings = Vec::new();
        let output = action
            .run(&mut |_| {}, &mut |w| warnings.push(w))
            .expect("StatusAction should succeed even if papers dir is missing");

        assert_eq!(output.papers.items.len(), 0);
        assert_eq!(warnings.len(), 1);

        let expected_path = project_root.join("papers");
        assert!(matches!(&warnings[0], StatusWarning::PapersDirNotFound(p) if p == &expected_path));
    }

    #[test]
    fn test_status_action_warns_on_missing_templates_and_dist_dirs() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path().to_path_buf();
        std::fs::create_dir_all(project_root.join("papers")).unwrap();

        let loaded = dummy_loaded_project(project_root.clone());
        let action = StatusAction::new(loaded, dummy_toolchain());
        let mut warnings = Vec::new();

        let output = action.run(&mut |_| {}, &mut |w| warnings.push(w)).unwrap();

        assert_eq!(output.templates.items, Vec::<String>::new());
        assert_eq!(output.dist.root, project_root.join("dist"));
        assert_eq!(
            warnings,
            vec![
                StatusWarning::TemplatesDirNotFound(project_root.join("templates")),
                StatusWarning::DistDirNotFound(project_root.join("dist")),
            ]
        );
    }

    #[test]
    fn test_status_action_uses_status_error_when_papers_cannot_be_read() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path().to_path_buf();
        std::fs::write(project_root.join("papers"), b"not a directory").unwrap();

        let loaded = dummy_loaded_project(project_root);
        let action = StatusAction::new(loaded, dummy_toolchain());

        let errors = match action.run(&mut |_| {}, &mut |_| {}) {
            Ok(_) => panic!("expected status to fail when papers cannot be read"),
            Err(errors) => errors,
        };

        assert!(matches!(
            errors.as_slice(),
            [StatusError::PapersReadFailed(CollectionError::Io(_))]
        ));
    }

    #[test]
    fn test_status_action_uses_status_error_when_templates_cannot_be_read() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path().to_path_buf();
        std::fs::create_dir_all(project_root.join("papers")).unwrap();
        std::fs::write(project_root.join("templates"), b"not a directory").unwrap();

        let loaded = dummy_loaded_project(project_root);
        let action = StatusAction::new(loaded, dummy_toolchain());

        let errors = match action.run(&mut |_| {}, &mut |_| {}) {
            Ok(_) => panic!("expected status to fail when templates cannot be read"),
            Err(errors) => errors,
        };

        assert!(matches!(
            errors.as_slice(),
            [StatusError::TemplatesReadFailed(CollectionError::Io(_))]
        ));
    }
}
