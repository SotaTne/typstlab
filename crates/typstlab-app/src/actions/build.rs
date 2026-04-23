use typstlab_proto::{Action, Entity, Collection, Artifact};
use crate::models::{Project, Paper, ManagedStore, BuildArtifact};
use crate::actions::resolve_typst::StoreError;
use crate::actions::discovery::{DiscoveryAction, DiscoveryError};
use typstlab_base::driver::{TypstDriver, TypstCommand};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("Discovery failure: {0:?}")]
    Discovery(Vec<DiscoveryError>),
    #[error("Environment failure: Failed to load project config: {0}")]
    ConfigLoadError(String),
    #[error("Environment failure: Resource resolution failed: {0:?}")]
    ResolutionError(Vec<StoreError>),
    #[error("Discovery failure: {0}")]
    GeneralDiscoveryError(String),
    #[error("No targets: No papers found to build")]
    NoTargetsFound,
    #[error("Build failed for artifact: {0:?}")]
    PaperBuildError(BuildArtifact),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub enum BuildEvent {
    ProjectLoaded { name: String },
    DiscoveryStarted { inputs: Vec<String> },
    ResolvingTypst { version: String },
    DiscoveredTargets { count: usize },
    Starting { paper_id: String },
    Finished { 
        artifact: BuildArtifact, 
        duration_ms: u64 
    },
}

pub struct BuildAction {
    pub project: Project,
    pub store: ManagedStore,
    pub inputs: Option<Vec<String>>,
}

impl BuildAction {
    pub fn new(project: Project, store: ManagedStore, inputs: Option<Vec<String>>) -> Self {
        Self { project, store, inputs }
    }
}

impl Action<(), BuildEvent, BuildError> for BuildAction {
    fn run(&self, monitor: &mut dyn FnMut(BuildEvent)) -> Result<(), Vec<BuildError>>
    {
        let mut errors = Vec::new();

        // 1. 設定のロード
        let config = match self.project.load_config() {
            Ok(c) => c,
            Err(e) => return Err(vec![BuildError::ConfigLoadError(e.to_string())]),
        };
            
        monitor(BuildEvent::ProjectLoaded { 
            name: config.project.name.clone() 
        });

        // 2. ターゲットの特定
        let targets: Vec<Paper> = if let Some(inputs) = &self.inputs {
            monitor(BuildEvent::DiscoveryStarted { inputs: inputs.clone() });
            let discovery = DiscoveryAction {
                scope: self.project.papers_scope(),
                inputs: inputs.clone(),
            };
            match discovery.run(&mut |_| {}) {
                Ok(t) => t,
                Err(e) => return Err(vec![BuildError::Discovery(e)]),
            }
        } else {
            match self.project.papers_scope().list() {
                Ok(t) => t,
                Err(e) => return Err(vec![BuildError::GeneralDiscoveryError(e.to_string())]),
            }
        };

        if targets.is_empty() {
            return Err(vec![BuildError::NoTargetsFound]);
        }

        monitor(BuildEvent::DiscoveredTargets { count: targets.len() });

        // 3. Typst 解決
        monitor(BuildEvent::ResolvingTypst { version: config.typst.version.clone() });
        let resolver = self.store.typst_resolver(&config.typst.version);
        let typst = match resolver.run(&mut |_| {}) {
            Ok(t) => t,
            Err(e) => return Err(vec![BuildError::ResolutionError(e)]),
        };
        let driver = TypstDriver::new(typst.path());

        // 4. 成果物領土の準備
        let artifact_scope = self.project.build_artifact_scope();

        // 5. 各ターゲットのビルド実行
        for paper in targets {
            monitor(BuildEvent::Starting { paper_id: paper.id.clone() });

            // 領土階層から成果物実体（Artifact）を生成
            let mut artifact = artifact_scope.paper_scope(&paper.id).format_artifact("pdf");
            
            if let Err(e) = std::fs::create_dir_all(artifact.path()) {
                errors.push(BuildError::IoError(e));
                continue;
            }

            let output_path = artifact.path().join(format!("{}.pdf", paper.output_base_name()));

            let command = TypstCommand::Compile {
                source: paper.main_typ_path(),
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
