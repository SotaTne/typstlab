use crate::actions::resolve_typst::{ResolveEvent, StoreError};
use crate::models::{Docs, ManagedStore, Project, Typst};
use std::path::PathBuf;
use thiserror::Error;
use typstlab_proto::Action;

#[derive(Error, Debug)]
pub enum BootstrapError {
    #[error("Failed to identify project: {0}")]
    ProjectError(String),
    #[error("Failed to prepare store: {0}")]
    StoreError(String),
    #[error("Asset resolution failed: {0:?}")]
    ResolutionError(Vec<StoreError>),
}

/// 起動プロセス中に発生するイベント
#[derive(Debug, Clone)]
pub enum BootstrapEvent {
    IdentifyingProject {
        root: PathBuf,
    },
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

/// 起動完了後に得られる「全能の鍵（コンテキスト）」
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
    fn run(
        &self,
        monitor: &mut dyn FnMut(BootstrapEvent),
    ) -> Result<AppContext, Vec<BootstrapError>> {
        // 1. プロジェクトの特定
        monitor(BootstrapEvent::IdentifyingProject {
            root: self.project_root.clone(),
        });
        let project = Project::new(self.project_root.clone());
        let config = project
            .load_config()
            .map_err(|e| vec![BootstrapError::ProjectError(e.to_string())])?;

        monitor(BootstrapEvent::ProjectReady {
            name: config.project.name.clone(),
        });

        // 2. ストアの準備
        monitor(BootstrapEvent::PreparingStore {
            cache_root: self.cache_root.clone(),
        });
        let store = ManagedStore::new(self.cache_root.clone());

        // 3. Typst 解決 (無ければ自動でダウンロードプロセスのトリガーになる)
        let version = config.typst.version.clone();
        let typst_resolver = store.typst_resolver(&version);
        let typst = typst_resolver
            .run(&mut |e| {
                monitor(BootstrapEvent::ResolvingTypst {
                    version: version.clone(),
                    event: e,
                });
            })
            .map_err(|e| vec![BootstrapError::ResolutionError(e)])?;

        // 4. Docs 解決 (同様に自動同期)
        let docs_resolver = store.docs_resolver(&version);
        let docs = docs_resolver
            .run(&mut |e| {
                monitor(BootstrapEvent::ResolvingDocs {
                    version: version.clone(),
                    event: e,
                });
            })
            .map_err(|e| vec![BootstrapError::ResolutionError(e)])?;

        monitor(BootstrapEvent::Ready);

        Ok(AppContext {
            project,
            store,
            typst,
            docs,
        })
    }
}
