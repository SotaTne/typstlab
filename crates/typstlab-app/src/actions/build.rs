use crate::actions::discovery::{DiscoveryAction, DiscoveryError};
use crate::models::{
    CollectionError, PaperError, PaperHandle, Project, ProjectConfig, ProjectHandle,
};
use thiserror::Error;
use typstlab_base::driver::{TypstCommand, TypstDriver};
use typstlab_proto::{Action, AppEvent, Collection, Entity, EventScope, Loadable, Loaded};

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("Discovery failure: {0:?}")]
    Discovery(Vec<DiscoveryError>),
    #[error("Discovery failure: {0}")]
    GeneralDiscoveryError(#[from] CollectionError),
    #[error("Build failed for artifact: {0:?}")]
    PaperBuildError(crate::models::BuildArtifact),
    #[error("Failed to load paper '{paper_id}': {source}")]
    PaperLoadError {
        paper_id: String,
        #[source]
        source: PaperError,
    },
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistObject {
    pub paper_id: String,
    pub pdf: Option<std::path::PathBuf>,
    pub png: Option<Vec<std::path::PathBuf>>,
    pub svg: Option<Vec<std::path::PathBuf>>,
    pub html: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildWarning {
    NoTargetsFound,
}

#[derive(Debug, Clone)]
pub enum BuildEvent {
    ProjectLoaded {
        name: String,
    },
    DiscoveryStarted {
        inputs: Vec<String>,
    },
    DiscoveredTargets {
        count: usize,
    },
    Starting {
        paper_id: String,
    },
    Finished {
        artifact: crate::models::BuildArtifact,
        duration_ms: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildFormat {
    pub pdf: bool,
    pub png: bool,
    pub svg: bool,
    pub html: bool,
}

impl Default for BuildFormat {
    fn default() -> Self {
        Self {
            pdf: true,
            png: false,
            svg: false,
            html: false,
        }
    }
}

impl BuildFormat {
    pub fn active_formats(&self) -> Vec<&'static str> {
        let mut formats = Vec::new();
        if self.pdf {
            formats.push("pdf");
        }
        if self.png {
            formats.push("png");
        }
        if self.svg {
            formats.push("svg");
        }
        if self.html {
            formats.push("html");
        }
        formats
    }
}

pub struct BuildAction {
    pub loaded_project: Loaded<Project, ProjectConfig>,
    pub typst_driver: TypstDriver,
    pub inputs: Option<Vec<String>>,
    pub format: BuildFormat,
}

impl BuildAction {
    pub fn new(
        loaded_project: Loaded<Project, ProjectConfig>,
        typst_driver: TypstDriver,
        inputs: Option<Vec<String>>,
        format: BuildFormat,
    ) -> Self {
        Self {
            loaded_project,
            typst_driver,
            inputs,
            format,
        }
    }
}

impl Action for BuildAction {
    type Output = Vec<DistObject>;
    type Event = BuildEvent;
    type Warning = BuildWarning;
    type Error = BuildError;

    fn run(
        self,
        monitor: &mut dyn FnMut(AppEvent<BuildEvent>),
        warning: &mut dyn FnMut(BuildWarning),
    ) -> Result<Self::Output, Vec<Self::Error>> {
        let mut errors = Vec::new();
        let scope = EventScope::new("build");

        monitor(AppEvent::verbose(
            scope.clone(),
            BuildEvent::ProjectLoaded {
                name: self.loaded_project.name().to_string(),
            },
        ));

        // 2. ターゲットの特定
        let targets = if let Some(inputs) = &self.inputs {
            monitor(AppEvent::verbose(
                scope.clone(),
                BuildEvent::DiscoveryStarted {
                    inputs: inputs.clone(),
                },
            ));
            let discovery =
                DiscoveryAction::new(self.loaded_project.papers_scope(), inputs.clone());
            match discovery.run(&mut |_| {}, &mut |_| {}) {
                Ok(t) => t,
                Err(e) => return Err(vec![BuildError::Discovery(e)]),
            }
        } else {
            match self.loaded_project.papers_scope().list() {
                Ok(t) => t,
                Err(e) => return Err(vec![BuildError::GeneralDiscoveryError(e)]),
            }
        };

        if targets.is_empty() {
            warning(BuildWarning::NoTargetsFound);
            return Ok(Vec::new());
        }

        monitor(AppEvent::line(
            scope.clone(),
            BuildEvent::DiscoveredTargets {
                count: targets.len(),
            },
        ));

        // 4. 成果物領土の準備
        let artifact_scope = self.loaded_project.build_artifact_scope();
        let mut results = Vec::new();

        // 5. 各ターゲットのビルド実行
        for paper in targets {
            let paper_id = paper.id.clone();
            let loaded_paper = match paper.load() {
                Ok(loaded_paper) => loaded_paper,
                Err(source) => {
                    errors.push(BuildError::PaperLoadError { paper_id, source });
                    continue;
                }
            };

            monitor(AppEvent::line(
                scope.clone(),
                BuildEvent::Starting {
                    paper_id: loaded_paper.paper_id().to_string(),
                },
            ));

            let mut dist_obj = DistObject {
                paper_id: loaded_paper.paper_id().to_string(),
                pdf: None,
                png: None,
                svg: None,
                html: None,
            };

            for fmt in self.format.active_formats() {
                // 領土階層から成果物実体（Artifact）を生成
                let mut artifact = artifact_scope
                    .paper_scope(loaded_paper.paper_id())
                    .format_artifact(fmt);

                // 以前の出力（特に複数ページの画像）をクリーンアップ
                if artifact.path().exists()
                    && let Err(e) = std::fs::remove_dir_all(artifact.path())
                {
                    errors.push(BuildError::IoError(e));
                    continue;
                }

                if let Err(e) = std::fs::create_dir_all(artifact.path()) {
                    errors.push(BuildError::IoError(e));
                    continue;
                }

                let output_filename = match fmt {
                    "pdf" => format!("{}.pdf", loaded_paper.output_base_name()),
                    "png" => "{0p}.png".to_string(),
                    "svg" => "{0p}.svg".to_string(),
                    "html" => format!("{}.html", loaded_paper.output_base_name()),
                    _ => unreachable!(),
                };

                let output_path = artifact.path().join(output_filename);
                let features = if fmt == "html" {
                    vec!["html".to_string()]
                } else {
                    vec![]
                };

                let command = TypstCommand::Compile {
                    source: loaded_paper.main_typ_path(),
                    output: Some(output_path.clone()),
                    features,
                };

                match self.typst_driver.execute(command) {
                    Ok(res) if res.exit_code == 0 => {
                        artifact.success = true;
                        monitor(AppEvent::line(
                            scope.clone(),
                            BuildEvent::Finished {
                                artifact: artifact.clone(),
                                duration_ms: res.duration_ms,
                            },
                        ));
                        match fmt {
                            "pdf" => dist_obj.pdf = Some(output_path),
                            "html" => dist_obj.html = Some(output_path),
                            "png" | "svg" => {
                                let mut files = Vec::new();
                                if let Ok(entries) = std::fs::read_dir(artifact.path()) {
                                    for entry in entries.flatten() {
                                        if entry.path().extension().and_then(|e| e.to_str())
                                            == Some(fmt)
                                        {
                                            files.push(entry.path());
                                        }
                                    }
                                }
                                files.sort();
                                if fmt == "png" {
                                    dist_obj.png = Some(files);
                                } else {
                                    dist_obj.svg = Some(files);
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(res) => {
                        artifact.success = false;
                        artifact.error_message = Some(res.stderr);
                        errors.push(BuildError::PaperBuildError(artifact));
                    }
                    Err(e) => {
                        artifact.success = false;
                        artifact.error_message = Some(e.to_string());
                        errors.push(BuildError::PaperBuildError(artifact));
                    }
                }
            }
            results.push(dist_obj);
        }

        if errors.is_empty() {
            Ok(results)
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BuildAction, BuildWarning};
    use crate::models::project::{ProjectInfo, StructureConfig};
    use crate::models::{Project, ProjectConfig, ProjectToolChain};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use typstlab_base::driver::TypstDriver;
    use typstlab_proto::{Action, Loaded};

    fn loaded_project(root: &std::path::Path) -> Loaded<Project, ProjectConfig> {
        Loaded {
            actual: Project::new(root.to_path_buf()),
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

    #[test]
    fn test_no_targets_emits_warning_and_succeeds() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("papers")).unwrap();
        fs::create_dir_all(temp.path().join("dist")).unwrap();

        let project = loaded_project(temp.path());
        let driver = TypstDriver::new(PathBuf::from("typst"));

        let action = BuildAction::new(project, driver, None, Default::default());
        let mut warnings = Vec::new();

        let result = action.run(&mut |_| {}, &mut |warning| warnings.push(warning));

        assert!(result.is_ok());
        assert_eq!(warnings, vec![BuildWarning::NoTargetsFound]);
    }

    #[test]
    fn test_build_format_active_formats() {
        let mut format = super::BuildFormat::default();
        assert_eq!(format.active_formats(), vec!["pdf"]);

        format.png = true;
        format.svg = true;
        format.html = true;
        assert_eq!(format.active_formats(), vec!["pdf", "png", "svg", "html"]);

        format.pdf = false;
        assert_eq!(format.active_formats(), vec!["png", "svg", "html"]);
    }
}
