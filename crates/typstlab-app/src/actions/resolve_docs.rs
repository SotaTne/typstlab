use std::path::PathBuf;

use thiserror::Error;
use typstlab_base::link_resolver::ResolvedLink;
use typstlab_base::project_docs::{ProjectDocs, sync_project_docs};
use typstlab_proto::{Action, Collection, Installer, Store};

use crate::actions::download_docs::{DownloadDocsAction, DownloadDocsError, DownloadDocsEvent};
use crate::actions::resolve_typst::{ResolveEvent, StoreError};
use crate::models::{Docs, DocsStore};

#[derive(Debug, Error)]
pub enum ResolveDocsError<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    #[error("store failed: {0}")]
    Store(#[from] StoreError),
    #[error("download failed: {0}")]
    Download(#[from] DownloadDocsError<E>),
    #[error("project docs sync failed: {0}")]
    Sync(#[from] typstlab_base::ProjectDocsSyncError),
    #[error("download or resolve docs failed")]
    NotFound,
}

pub struct ResolveDocsAction<I>
where
    I: Installer,
{
    pub project_root: PathBuf,
    pub store: DocsStore,
    pub version: String,
    pub installer: I,
    pub link: ResolvedLink,
}

impl<I> Action<Docs, ResolveEvent, (), ResolveDocsError<I::Error>> for ResolveDocsAction<I>
where
    I: Installer,
{
    fn run(
        self,
        monitor: &mut dyn FnMut(ResolveEvent),
        _warning: &mut dyn FnMut(()),
    ) -> Result<Docs, Vec<ResolveDocsError<I::Error>>> {
        self.run_inner(monitor).map_err(|error| vec![error])
    }
}

impl<I> ResolveDocsAction<I>
where
    I: Installer,
{
    fn run_inner(
        self,
        monitor: &mut dyn FnMut(ResolveEvent),
    ) -> Result<Docs, ResolveDocsError<I::Error>> {
        monitor(ResolveEvent::CheckingCache);

        if let Some(docs) = self.store.resolve(&self.version)? {
            monitor(ResolveEvent::CacheHit);
            let synced = sync_project_docs(&self.project_root, ProjectDocs::Typst, docs.path)?;
            monitor(ResolveEvent::Completed);
            return Ok(Docs::new(synced));
        }

        monitor(ResolveEvent::CacheMiss);

        let download = DownloadDocsAction {
            installer: self.installer,
            store: self.store.clone(),
            version: self.version.clone(),
            link: self.link,
        };
        let staging = download
            .run(
                &mut |event| match event {
                    DownloadDocsEvent::Downloading { .. } => {}
                    DownloadDocsEvent::Transforming => {}
                },
                &mut |_| {},
            )
            .map_err(|errors| {
                errors
                    .into_iter()
                    .next()
                    .map(ResolveDocsError::Download)
                    .unwrap_or(ResolveDocsError::NotFound)
            })?;

        let docs = self.store.commit_staged(&self.version, staging)?;
        let synced = sync_project_docs(&self.project_root, ProjectDocs::Typst, docs.path)?;
        monitor(ResolveEvent::Completed);
        Ok(Docs::new(synced))
    }
}
