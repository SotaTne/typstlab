use crate::actions::resolve_docs::{ResolveDocsAction, ResolveDocsError};
use crate::actions::resolve_typst::{ResolveEvent, ResolveTypstAction, ResolveTypstError};
use crate::models::{Docs, DocsStore, ProjectToolChain, Typst, TypstStore};
use std::path::{Path, PathBuf};
use thiserror::Error;
use typstlab_base::VersionResolveError;
use typstlab_base::install::{
    DocsInstallError, DocsInstaller, HttpProvider, TypstInstallError, TypstInstaller,
};
use typstlab_base::link_resolver::{
    DocsLinkRequest, LinkResolveError, TypstLinkRequest, Version, resolve_docs_link,
    resolve_typst_link,
};
use typstlab_base::platform::Platform;
use typstlab_base::resolve_toolchain;
use typstlab_proto::{Action, AppEvent};

pub struct ToolchainResolveInput {
    pub project_root: PathBuf,
    pub toolchain: ProjectToolChain,
    pub typst_store: TypstStore,
    pub docs_store: DocsStore,
}

#[derive(Debug, Clone)]
pub struct ToolChain {
    pub typst: Typst,
    pub typst_docs: Option<Docs>,
    pub typst_docs_cache: Option<PathBuf>,
    pub typstyle: Option<()>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolchainResolveEvent {
    ResolvingTypst {
        version: String,
        event: ResolveEvent,
    },
    ResolvingDocs {
        version: String,
        event: ResolveEvent,
    },
    Completed,
}

#[derive(Error, Debug)]
pub enum ToolchainResolveError {
    #[error("Toolchain version resolution failed: {0}")]
    VersionResolution(#[from] VersionResolveError),
    #[error("Typst link resolution failed: {0}")]
    TypstLinkResolution(#[from] LinkResolveError),
    #[error("Failed to initialize Typst HTTP provider: {0}")]
    TypstInstallInit(reqwest::Error),
    #[error("Typst resolution failed: {0:?}")]
    TypstResolution(Vec<ResolveTypstError<TypstInstallError>>),
    #[error("Failed to initialize HTTP provider: {0}")]
    DocsInstallInit(reqwest::Error),
    #[error("Docs resolution failed: {0:?}")]
    DocsResolution(Vec<ResolveDocsError<DocsInstallError>>),
}

pub struct ToolchainResolveAction {
    pub input: ToolchainResolveInput,
}

impl Action for ToolchainResolveAction {
    type Output = ToolChain;
    type Event = ToolchainResolveEvent;
    type Warning = ();
    type Error = ToolchainResolveError;

    fn run(
        self,
        monitor: &mut dyn FnMut(AppEvent<Self::Event>),
        _warning: &mut dyn FnMut(Self::Warning),
    ) -> Result<Self::Output, Vec<Self::Error>> {
        let toolchain = self.resolve(monitor).map_err(|error| vec![error])?;
        monitor(AppEvent::line(
            typstlab_proto::EventScope::labeled("toolchain_resolve", "done"),
            ToolchainResolveEvent::Completed,
        ));
        Ok(toolchain)
    }
}

impl ToolchainResolveAction {
    fn resolve(
        self,
        monitor: &mut dyn FnMut(AppEvent<ToolchainResolveEvent>),
    ) -> Result<ToolChain, ToolchainResolveError> {
        let ToolchainResolveInput {
            project_root,
            toolchain,
            typst_store,
            docs_store,
        } = self.input;
        let resolved_toolchain = resolve_toolchain(&toolchain)?;

        let typst = Self::resolve_typst(&typst_store, resolved_toolchain.typst, monitor)?;
        let typst_docs_version = resolved_toolchain.typst_docs.clone();
        let typst_docs = Self::resolve_typst_docs(
            &project_root,
            &docs_store,
            typst_docs_version.clone(),
            monitor,
        )?;
        let typst_docs_cache = typst_docs_version.map(|_| docs_store.root.clone());

        Ok(ToolChain {
            typst,
            typst_docs,
            typst_docs_cache,
            typstyle: None,
        })
    }

    fn resolve_typst(
        typst_store: &TypstStore,
        version: String,
        monitor: &mut dyn FnMut(AppEvent<ToolchainResolveEvent>),
    ) -> Result<Typst, ToolchainResolveError> {
        let platform = Platform::current();
        let typst_link = resolve_typst_link(TypstLinkRequest {
            platform,
            version: Version::new(&version),
        })?;

        let typst_installer = TypstInstaller::new(
            HttpProvider::try_new().map_err(ToolchainResolveError::TypstInstallInit)?,
        );
        let typst_resolver = ResolveTypstAction {
            store: typst_store.clone(),
            version: version.clone(),
            installer: typst_installer,
            link: typst_link,
        };

        typst_resolver
            .run(
                &mut |event| {
                    monitor(
                        event.map_payload(|event| ToolchainResolveEvent::ResolvingTypst {
                            version: version.clone(),
                            event,
                        }),
                    );
                },
                &mut |_| {},
            )
            .map_err(ToolchainResolveError::TypstResolution)
    }

    fn resolve_typst_docs(
        project_root: &Path,
        docs_store: &DocsStore,
        version: Option<String>,
        monitor: &mut dyn FnMut(AppEvent<ToolchainResolveEvent>),
    ) -> Result<Option<Docs>, ToolchainResolveError> {
        let Some(version) = version else {
            return Ok(None);
        };

        let docs_link = resolve_docs_link(DocsLinkRequest {
            version: Version::new(&version),
        });
        let docs_installer = DocsInstaller::new(
            HttpProvider::try_new().map_err(ToolchainResolveError::DocsInstallInit)?,
        );
        let docs_resolver = ResolveDocsAction {
            project_root: project_root.to_path_buf(),
            store: docs_store.clone(),
            version: version.clone(),
            installer: docs_installer,
            link: docs_link,
        };

        docs_resolver
            .run(
                &mut |event| {
                    monitor(
                        event.map_payload(|event| ToolchainResolveEvent::ResolvingDocs {
                            version: version.clone(),
                            event,
                        }),
                    );
                },
                &mut |_| {},
            )
            .map_err(ToolchainResolveError::DocsResolution)
            .map(Some)
    }
}
