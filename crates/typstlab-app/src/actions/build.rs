use typstlab_proto::{Action, Entity, Collection};
use crate::models::{Project, Paper, ManagedStore};
use crate::actions::resolve_typst::StoreError;
use crate::actions::discovery::DiscoveryError;
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
    #[error("Discovery failure: Failed to list target papers: {0}")]
    DiscoveryError(String),
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
    ResolvingTypst { version: String },
    DiscoveredTargets { count: usize },
    Starting { paper_id: String },
    Finished { paper_id: String, output_path: PathBuf },
}

pub struct BuildAction {
    pub project: Project,
    pub store: ManagedStore,
    pub targets: Option<Vec<Paper>>,
}

impl BuildAction {
    pub fn new(project: Project, store: ManagedStore, targets: Option<Vec<Paper>>) -> Self {
        Self { project, store, targets }
    }
}

impl Action<(), BuildEvent, BuildError> for BuildAction {
    fn run(&self, monitor: &mut dyn FnMut(BuildEvent)) -> Result<(), Vec<BuildError>>
    {
        let mut errors = Vec::new();

        // 1. 環境の準備
        let config = match self.project.load_config() {
            Ok(c) => c,
            Err(e) => return Err(vec![BuildError::ConfigLoadError(e.to_string())]),
        };
            
        monitor(BuildEvent::ProjectLoaded { 
            name: config.project.name.clone() 
        });

        // 2. ターゲットの確定
        let targets: Vec<Paper> = if let Some(t) = &self.targets {
            t.clone()
        } else {
            let scope = self.project.papers_scope();
            match scope.list() {
                Ok(t) => t,
                Err(e) => return Err(vec![BuildError::DiscoveryError(e.to_string())]),
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

        // 4. 各ターゲットのビルド実行
        for paper in targets {
            monitor(BuildEvent::Starting { paper_id: paper.id.clone() });

            let command = TypstCommand::Compile {
                source: paper.main_typ_path(),
                output: None,
            };

            match driver.execute(command) {
                Ok(res) if res.exit_code == 0 => {
                    monitor(BuildEvent::Finished {
                        paper_id: paper.id.clone(),
                        output_path: paper.path().join("main.pdf"),
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
