use std::path::PathBuf;
use typstlab_proto::Action;
use crate::models::{Project, ManagedStore, Typst, Docs};
use crate::actions::resolve_typst::{StoreError, ResolveEvent};
use crate::actions::load::{LoadAction, LoadEvent};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BootstrapError {
    #[error("Failed to load project: {0}")]
    ProjectLoadError(String),
    #[error("Asset resolution failed: {0:?}")]
    ResolutionError(Vec<StoreError>),
}

/// 起動プロセス中に発生するイベント
#[derive(Debug, Clone)]
pub enum BootstrapEvent {
    IdentifyingProject { root: PathBuf },
    ProjectLoading(LoadEvent), // LoadAction のイベントを内包
    ProjectReady { name: String },
    PreparingStore { cache_root: PathBuf },
    ResolvingTypst { version: String, event: ResolveEvent },
    ResolvingDocs { version: String, event: ResolveEvent },
    Ready,
}

pub struct AppContext {
    pub project: Project,
    pub store: ManagedStore,
    pub typst: Typst,
    pub docs: Docs,
}

pub struct BootstrapAction {
    pub project_root: PathBuf,
    pub cache_root: PathBuf,
}

impl Action<AppContext, BootstrapEvent, BootstrapError> for BootstrapAction {
    fn run(self, monitor: &mut dyn FnMut(BootstrapEvent)) -> Result<AppContext, Vec<BootstrapError>> {
        // 1. プロジェクトのロード (LoadAction を使用)
        monitor(BootstrapEvent::IdentifyingProject { root: self.project_root.clone() });
        let project_model = Project::new(self.project_root);
        
        let load_action = LoadAction { target: project_model };
        let project = load_action.run(&mut |e| {
            monitor(BootstrapEvent::ProjectLoading(e));
        }).map_err(|errs| errs.into_iter().map(|e| BootstrapError::ProjectLoadError(e.to_string())).collect::<Vec<_>>())?;

        let config = project.config.as_ref().unwrap().clone();
        monitor(BootstrapEvent::ProjectReady { name: config.project.name.clone() });

        // 2. ストアの準備
        monitor(BootstrapEvent::PreparingStore { cache_root: self.cache_root.clone() });
        let store = ManagedStore::new(self.cache_root);

        // 3. Typst 解決
        let version = config.typst.version.clone();
        let typst_resolver = store.typst_resolver(&version);
        let typst = typst_resolver.run(&mut |e| {
            monitor(BootstrapEvent::ResolvingTypst { version: version.clone(), event: e });
        }).map_err(|errs| vec![BootstrapError::ResolutionError(errs)])?;

        // 4. Docs 解決
        let docs_resolver = store.docs_resolver(&version);
        let docs = docs_resolver.run(&mut |e| {
            monitor(BootstrapEvent::ResolvingDocs { version: version.clone(), event: e });
        }).map_err(|errs| vec![BootstrapError::ResolutionError(errs)])?;

        monitor(BootstrapEvent::Ready);

        Ok(AppContext {
            project,
            store,
            typst,
            docs,
        })
    }
}
