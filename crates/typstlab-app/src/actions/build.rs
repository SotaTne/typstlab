use typstlab_proto::{Action, Entity, Collection};
use crate::models::{Project, ManagedStore};
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
    #[error("Execution failure for '{paper_id}': {error}")]
    PaperBuildError {
        paper_id: String,
        error: String,
    },
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
        paper_id: String, 
        output_path: PathBuf, 
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
        let targets = if let Some(inputs) = &self.inputs {
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

            // 出力先の決定 (dist/paper_id/output_name.pdf)
            let paper_dist_root = artifact_scope.paper_dist_path(&paper.id);
            
            // ディレクトリの作成を保証
            if let Err(e) = std::fs::create_dir_all(&paper_dist_root) {
                errors.push(BuildError::IoError(e));
                continue;
            }

            // ベース名 + .pdf
            let output_filename = format!("{}.pdf", paper.output_base_name());
            let output_path = paper_dist_root.join(output_filename);

            let command = TypstCommand::Compile {
                source: paper.main_typ_path(),
                output: Some(output_path.clone()),
            };

            match driver.execute(command) {
                Ok(res) if res.exit_code == 0 => {
                    monitor(BuildEvent::Finished {
                        paper_id: paper.id.clone(),
                        output_path,
                        duration_ms: res.duration_ms,
                    });
                }
                Ok(res) => {
                    errors.push(BuildError::PaperBuildError {
                        paper_id: paper.id.clone(),
                        error: res.stderr,
                    });
                }
                Err(e) => {
                    errors.push(BuildError::PaperBuildError {
                        paper_id: paper.id.clone(),
                        error: e.to_string(),
                    });
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
