use crate::actions::load::{LoadAction, LoadEvent};
use crate::actions::resolve_docs::ResolveDocsAction;
use crate::actions::resolve_typst::{ResolveEvent, StoreError};
use crate::models::{
    Docs, DocsStore, Project, ProjectConfig, ProjectError, ProjectHandle, Typst, TypstStore,
};
use std::path::PathBuf;
use thiserror::Error;
use typstlab_base::install::{DocsInstaller, HttpProvider};
use typstlab_base::link_resolver::{DocsLinkRequest, resolve_docs_link};
use typstlab_base::version_resolver::resolve_versions_from_typst;
use typstlab_proto::Loaded;
use typstlab_proto::{Action, AppEvent, EventScope};

#[derive(Error, Debug)]
pub enum BootstrapError {
    #[error("Failed to load project: {0}")]
    ProjectLoadError(#[from] ProjectError),
    #[error("Asset resolution failed: {0:?}")]
    ResolutionError(Vec<StoreError>),
    #[error("Docs resolution failed: {0}")]
    DocsResolutionError(String),
    #[error("Failed to initialize HTTP provider: {0}")]
    DocsInstallInitError(String),
}

/// 起動プロセス中に発生するイベント
#[derive(Debug, Clone)]
pub enum BootstrapEvent {
    IdentifyingProject {
        root: PathBuf,
    },
    ProjectLoading(LoadEvent),
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
    pub typst_store: TypstStore,
    pub docs_store: DocsStore,
    pub typst: Typst,
    pub docs: Docs,
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

        // 3. Typst 解決
        let version = loaded_project.typst_version().to_string();
        let typst_resolver = crate::actions::resolve_typst::ResolveTypstAction {
            store_root: typst_store.root.clone(),
            version: version.clone(),
        };
        let typst = typst_resolver
            .run(
                &mut |e| {
                    monitor(e.map_payload(|event| BootstrapEvent::ResolvingTypst {
                        version: version.clone(),
                        event,
                    }));
                },
                &mut |_| {},
            )
            .map_err(|errs| vec![BootstrapError::ResolutionError(errs)])?;

        // 4. Docs 解決
        let versions = resolve_versions_from_typst(&version);
        let docs_link = resolve_docs_link(DocsLinkRequest { versions });
        let docs_installer = DocsInstaller::new(
            HttpProvider::try_new()
                .map_err(|error| vec![BootstrapError::DocsInstallInitError(error.to_string())])?,
        );
        let docs_resolver = ResolveDocsAction {
            project_root,
            store: docs_store.clone(),
            version: version.clone(),
            installer: docs_installer,
            link: docs_link,
        };
        let docs = docs_resolver
            .run(
                &mut |e| {
                    monitor(e.map_payload(|event| BootstrapEvent::ResolvingDocs {
                        version: version.clone(),
                        event,
                    }));
                },
                &mut |_| {},
            )
            .map_err(|errs| {
                vec![BootstrapError::DocsResolutionError(
                    errs.into_iter()
                        .map(|err| err.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                )]
            })?;

        monitor(AppEvent::line(scope, BootstrapEvent::Ready));

        Ok(AppContext {
            loaded_project,
            typst_store,
            docs_store,
            typst,
            docs,
        })
    }
}
