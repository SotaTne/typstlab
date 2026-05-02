use crate::actions::load::{LoadAction, LoadEvent};
use crate::actions::toolchain_resolve::{
    ToolChain, ToolchainResolveAction, ToolchainResolveError, ToolchainResolveEvent,
    ToolchainResolveInput,
};
use crate::models::{DocsStore, Project, ProjectConfig, ProjectError, ProjectHandle, TypstStore};
use std::path::PathBuf;
use thiserror::Error;
use typstlab_proto::Loaded;
use typstlab_proto::{Action, AppEvent, EventScope};

#[derive(Error, Debug)]
pub enum BootstrapError {
    #[error("Failed to load project: {0}")]
    ProjectLoadError(#[from] ProjectError),
    #[error("Toolchain resolution failed: {0:?}")]
    ToolchainResolve(Vec<ToolchainResolveError>),
}

/// 起動プロセス中に発生するイベント
#[derive(Debug, Clone)]
pub enum BootstrapEvent {
    IdentifyingProject { root: PathBuf },
    ProjectLoading(LoadEvent),
    ProjectReady { name: String },
    PreparingStore { cache_root: PathBuf },
    ResolvingToolchain(ToolchainResolveEvent),
    Ready,
}

pub struct AppContext {
    pub loaded_project: Loaded<Project, ProjectConfig>,
    pub typst_store: TypstStore,
    pub docs_store: DocsStore,
    pub toolchain: ToolChain,
}

pub struct BootstrapAction {
    pub project_root: PathBuf,
    pub cache_root: PathBuf,
}

impl Action for BootstrapAction {
    type Output = AppContext;
    type Event = BootstrapEvent;
    type Warning = ();
    type Error = BootstrapError;

    fn run(
        self,
        monitor: &mut dyn FnMut(AppEvent<BootstrapEvent>),
        _warning: &mut dyn FnMut(()),
    ) -> Result<Self::Output, Vec<Self::Error>> {
        let scope = EventScope::new("bootstrap");
        // 1. プロジェクトのロード
        monitor(AppEvent::verbose(
            scope.clone(),
            BootstrapEvent::IdentifyingProject {
                root: self.project_root.clone(),
            },
        ));
        let project_root = self.project_root.clone();
        let project_model = Project::new(project_root.clone());

        let load_action = LoadAction {
            target: project_model,
        };
        let loaded_project: Loaded<Project, ProjectConfig> = load_action
            .run(
                &mut |e| {
                    monitor(e.map_payload(BootstrapEvent::ProjectLoading));
                },
                &mut |_| {},
            )
            .map_err(|errs| {
                errs.into_iter()
                    .map(BootstrapError::ProjectLoadError)
                    .collect::<Vec<_>>()
            })?;

        monitor(AppEvent::line(
            scope.clone(),
            BootstrapEvent::ProjectReady {
                name: loaded_project.name().to_string(),
            },
        ));

        // 2. ストアの準備
        monitor(AppEvent::verbose(
            scope.clone(),
            BootstrapEvent::PreparingStore {
                cache_root: self.cache_root.clone(),
            },
        ));
        let typst_store = TypstStore::new(self.cache_root.join("typst"));
        let docs_store = DocsStore::new(self.cache_root.join("docs"));

        // 3. Toolchain 解決
        let toolchain_action = ToolchainResolveAction {
            input: ToolchainResolveInput {
                project_root: project_root.clone(),
                toolchain: loaded_project.toolchain().clone(),
                typst_store: typst_store.clone(),
                docs_store: docs_store.clone(),
            },
        };
        let toolchain = toolchain_action
            .run(
                &mut |e| {
                    monitor(e.map_payload(BootstrapEvent::ResolvingToolchain));
                },
                &mut |_| {},
            )
            .map_err(|errs| vec![BootstrapError::ToolchainResolve(errs)])?;

        monitor(AppEvent::line(scope, BootstrapEvent::Ready));

        Ok(AppContext {
            loaded_project,
            typst_store,
            docs_store,
            toolchain,
        })
    }
}
