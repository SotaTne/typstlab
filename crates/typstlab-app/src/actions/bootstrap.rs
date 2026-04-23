use crate::actions::load::{LoadAction, LoadEvent};
use crate::actions::resolve_typst::{ResolveEvent, StoreError};
use crate::models::{
    Docs, ManagedStore, Project, ProjectConfig, ProjectError, ProjectHandle, Typst,
};
use std::path::PathBuf;
use thiserror::Error;
use typstlab_proto::Action;
use typstlab_proto::Loaded;

#[derive(Error, Debug)]
pub enum BootstrapError {
    #[error("Failed to load project: {0}")]
    ProjectLoadError(#[from] ProjectError),
    #[error("Asset resolution failed: {0:?}")]
    ResolutionError(Vec<StoreError>),
}

/// 起動プロセス中に発生するイベント
#[derive(Debug, Clone)]
pub enum BootstrapEvent {
    IdentifyingProject {
        root: PathBuf,
    },
    ProjectLoading(LoadEvent), // LoadAction のイベントを内包
    ProjectReady {
        name: String,
    },
    PreparingStore {
        cache_root: PathBuf,
    },
    ResolvingTypst {
        version: String,
        event: ResolveEvent,
    },
    ResolvingDocs {
        version: String,
        event: ResolveEvent,
    },
    Ready,
}

pub struct AppContext {
    pub loaded_project: Loaded<Project, ProjectConfig>,
    pub store: ManagedStore,
    pub typst: Typst,
    pub docs: Docs,
}

pub struct BootstrapAction {
    pub project_root: PathBuf,
    pub cache_root: PathBuf,
}

impl Action<AppContext, BootstrapEvent, BootstrapError> for BootstrapAction {
    fn run(
        self,
        monitor: &mut dyn FnMut(BootstrapEvent),
    ) -> Result<AppContext, Vec<BootstrapError>> {
        // 1. プロジェクトのロード (LoadAction を使用)
        monitor(BootstrapEvent::IdentifyingProject {
            root: self.project_root.clone(),
        });
        let project_model = Project::new(self.project_root);

        let load_action = LoadAction {
            target: project_model,
        };
        let loaded_project: Loaded<Project, ProjectConfig> = load_action
            .run(&mut |e| {
                monitor(BootstrapEvent::ProjectLoading(e));
            })
            .map_err(|errs| {
                errs.into_iter()
                    .map(BootstrapError::ProjectLoadError)
                    .collect::<Vec<_>>()
            })?;

        monitor(BootstrapEvent::ProjectReady {
            name: loaded_project.name().to_string(),
        });

        // 2. ストアの準備
        monitor(BootstrapEvent::PreparingStore {
            cache_root: self.cache_root.clone(),
        });
        let store = ManagedStore::new(self.cache_root);

        // 3. Typst 解決
        let version = loaded_project.typst_version().to_string();
        let typst_resolver = store.typst_resolver(&version);
        let typst = typst_resolver
            .run(&mut |e| {
                monitor(BootstrapEvent::ResolvingTypst {
                    version: version.clone(),
                    event: e,
                });
            })
            .map_err(|errs| vec![BootstrapError::ResolutionError(errs)])?;

        // 4. Docs 解決
        let docs_resolver = store.docs_resolver(&version);
        let docs = docs_resolver
            .run(&mut |e| {
                monitor(BootstrapEvent::ResolvingDocs {
                    version: version.clone(),
                    event: e,
                });
            })
            .map_err(|errs| vec![BootstrapError::ResolutionError(errs)])?;

        monitor(BootstrapEvent::Ready);

        Ok(AppContext {
            loaded_project,
            store,
            typst,
            docs,
        })
    }
}
