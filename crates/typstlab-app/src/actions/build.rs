use crate::actions::discovery::{DiscoveryAction, DiscoveryError};
use crate::actions::resolve_typst::StoreError;
use crate::models::{
    CollectionError, ManagedStore, PaperError, PaperHandle, Project, ProjectConfig,
    ProjectHandle,
};
use thiserror::Error;
use typstlab_base::driver::{TypstCommand, TypstDriver};
use typstlab_proto::{Action, Collection, Entity, Loadable, Loaded};

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("Discovery failure: {0:?}")]
    Discovery(Vec<DiscoveryError>),
    #[error("Environment failure: Resource resolution failed: {0:?}")]
    ResolutionError(Vec<StoreError>),
    #[error("Discovery failure: {0}")]
    GeneralDiscoveryError(#[from] CollectionError),
    #[error("No targets: No papers found to build")]
    NoTargetsFound,
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

#[derive(Debug, Clone)]
pub enum BuildEvent {
    ProjectLoaded {
        name: String,
    },
    DiscoveryStarted {
        inputs: Vec<String>,
    },
    ResolvingTypst {
        version: String,
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

pub struct BuildAction {
    pub loaded_project: Loaded<Project, ProjectConfig>,
    pub store: ManagedStore,
    pub inputs: Option<Vec<String>>,
}

impl BuildAction {
    pub fn new(
        loaded_project: Loaded<Project, ProjectConfig>,
        store: ManagedStore,
        inputs: Option<Vec<String>>,
    ) -> Self {
        Self {
            loaded_project,
            store,
            inputs,
        }
    }
}

impl Action<(), BuildEvent, BuildError> for BuildAction {
    fn run(self, monitor: &mut dyn FnMut(BuildEvent)) -> Result<(), Vec<BuildError>> {
        let mut errors = Vec::new();

        monitor(BuildEvent::ProjectLoaded {
            name: self.loaded_project.name().to_string(),
        });

        // 2. ターゲットの特定
        let targets = if let Some(inputs) = &self.inputs {
            monitor(BuildEvent::DiscoveryStarted {
                inputs: inputs.clone(),
            });
            let discovery = DiscoveryAction {
                scope: self.loaded_project.papers_scope(),
                inputs: inputs.clone(),
            };
            match discovery.run(&mut |_| {}) {
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
            return Err(vec![BuildError::NoTargetsFound]);
        }

        monitor(BuildEvent::DiscoveredTargets {
            count: targets.len(),
        });

        // 3. Typst 解決
        let version = self.loaded_project.typst_version().to_string();
        monitor(BuildEvent::ResolvingTypst {
            version: version.clone(),
        });
        let resolver = self.store.typst_resolver(&version);
        let typst = match resolver.run(&mut |_| {}) {
            Ok(t) => t,
            Err(e) => return Err(vec![BuildError::ResolutionError(e)]),
        };
        let driver = TypstDriver::new(typst.path());

        // 4. 成果物領土の準備
        let artifact_scope = self.loaded_project.build_artifact_scope();

        // 5. 各ターゲットのビルド実行
        for paper in targets {
            let paper_id = paper.id.clone();
            let loaded_paper = match paper.load() {
                Ok(loaded_paper) => loaded_paper,
                Err(source) => {
                    errors.push(BuildError::PaperLoadError {
                        paper_id,
                        source,
                    });
                    continue;
                }
            };

            monitor(BuildEvent::Starting {
                paper_id: loaded_paper.paper_id().to_string(),
            });

            // 領土階層から成果物実体（Artifact）を生成
            let mut artifact = artifact_scope
                .paper_scope(loaded_paper.paper_id())
                .format_artifact("pdf");

            if let Err(e) = std::fs::create_dir_all(artifact.path()) {
                errors.push(BuildError::IoError(e));
                continue;
            }

            let output_path = artifact
                .path()
                .join(format!("{}.pdf", loaded_paper.output_base_name()));

            let command = TypstCommand::Compile {
                source: loaded_paper.main_typ_path(),
                output: Some(output_path),
            };

            match driver.execute(command) {
                Ok(res) if res.exit_code == 0 => {
                    artifact.success = true;
                    monitor(BuildEvent::Finished {
                        artifact,
                        duration_ms: res.duration_ms,
                    });
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

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
