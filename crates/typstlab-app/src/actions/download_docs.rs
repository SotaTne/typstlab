use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use thiserror::Error;
use typstlab_base::RAW_DOCS_FILENAME;
use typstlab_base::docs_parser::{self, DocsRenderError};
use typstlab_base::link_resolver::ResolvedLink;
use typstlab_proto::{Action, Installer, Store};

use crate::actions::resolve_typst::StoreError;
use crate::models::{Docs, DocsStore};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadDocsEvent {
    Downloading { current: u64, total: u64 },
    Transforming,
}

#[derive(Debug, Error)]
pub enum DownloadDocsError<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    #[error("failed to install docs source: {0}")]
    Install(#[source] E),
    #[error("docs installer did not produce raw file: {path}")]
    RawFileMissing { path: PathBuf },
    #[error("failed to open raw docs file '{path}': {source}")]
    RawFileOpenFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to render docs: {0}")]
    Render(#[from] DocsRenderError),
    #[error("failed to create docs staging area: {0}")]
    Store(#[source] StoreError),
    #[error("failed to copy rendered docs into staging: {0}")]
    Copy(#[source] std::io::Error),
}

pub struct DownloadDocsAction<I>
where
    I: Installer,
{
    pub installer: I,
    pub store: DocsStore,
    pub version: String,
    pub link: ResolvedLink,
}

impl<I>
    Action<
        <DocsStore as Store<Docs, StoreError>>::Staging,
        DownloadDocsEvent,
        (),
        DownloadDocsError<I::Error>,
    > for DownloadDocsAction<I>
where
    I: Installer,
{
    fn run(
        self,
        monitor: &mut dyn FnMut(DownloadDocsEvent),
        _warning: &mut dyn FnMut(()),
    ) -> Result<<DocsStore as Store<Docs, StoreError>>::Staging, Vec<DownloadDocsError<I::Error>>>
    {
        self.run_inner(monitor).map_err(|error| vec![error])
    }
}

impl<I> DownloadDocsAction<I>
where
    I: Installer,
{
    fn run_inner(
        self,
        monitor: &mut dyn FnMut(DownloadDocsEvent),
    ) -> Result<<DocsStore as Store<Docs, StoreError>>::Staging, DownloadDocsError<I::Error>> {
        let progress_events = Arc::new(Mutex::new(Vec::new()));
        let progress_writer = Arc::clone(&progress_events);
        let installation = self
            .installer
            .install(&self.link.url, self.link.format, move |current, total| {
                if let Ok(mut events) = progress_writer.lock() {
                    events.push(DownloadDocsEvent::Downloading { current, total });
                }
            })
            .map_err(DownloadDocsError::Install)?;
        if let Ok(events) = progress_events.lock() {
            for event in events.iter().cloned() {
                monitor(event);
            }
        }

        let raw_path = installation.as_ref().join(RAW_DOCS_FILENAME);
        if !raw_path.exists() {
            return Err(DownloadDocsError::RawFileMissing { path: raw_path });
        }

        let raw = File::open(&raw_path).map_err(|source| DownloadDocsError::RawFileOpenFailed {
            path: raw_path,
            source,
        })?;

        monitor(DownloadDocsEvent::Transforming);
        let rendered = docs_parser::render_docs_from_reader(BufReader::new(raw))?;
        let staging = self
            .store
            .create_staging_area(&self.version)
            .map_err(DownloadDocsError::Store)?;

        copy_dir_contents(rendered.path(), staging.as_ref()).map_err(DownloadDocsError::Copy)?;
        Ok(staging)
    }
}

pub(crate) fn copy_dir_contents(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        if source.is_dir() {
            copy_dir_contents(&source, &target)?;
        } else {
            std::fs::copy(&source, &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use typstlab_proto::{Collection, SourceFormat};

    use super::*;

    #[derive(Debug, Error)]
    #[error("fake install failed")]
    struct FakeInstallError;

    #[derive(Clone)]
    struct FakeDocsInstaller {
        raw_docs: &'static str,
    }

    impl FakeDocsInstaller {
        fn new(raw_docs: &'static str) -> Self {
            Self { raw_docs }
        }
    }

    impl Installer for FakeDocsInstaller {
        type Error = FakeInstallError;
        type Installation = TempDir;

        fn install<F>(
            &self,
            url: &str,
            format: SourceFormat,
            mut on_progress: F,
        ) -> Result<Self::Installation, Self::Error>
        where
            F: FnMut(u64, u64) + Send + 'static,
        {
            assert_eq!(url, "https://example.com/docs.json");
            assert_eq!(format, SourceFormat::Raw);

            let installation = TempDir::new().unwrap();
            std::fs::write(installation.path().join(RAW_DOCS_FILENAME), self.raw_docs).unwrap();
            on_progress(self.raw_docs.len() as u64, self.raw_docs.len() as u64);
            Ok(installation)
        }
    }

    fn docs_json() -> &'static str {
        r#"[
            {
                "route": "/DOCS-BASE/",
                "title": "Overview",
                "body": { "kind": "html", "content": "<p>Hello docs</p>" },
                "children": [
                    {
                        "route": "/DOCS-BASE/tutorial/writing/",
                        "title": "Writing",
                        "body": { "kind": "html", "content": "<p>Write text</p>" },
                        "children": []
                    }
                ]
            }
        ]"#
    }

    fn link() -> ResolvedLink {
        ResolvedLink {
            url: "https://example.com/docs.json".to_string(),
            format: SourceFormat::Raw,
        }
    }

    #[test]
    fn test_download_docs_action_returns_commit_ready_staging_without_committing() {
        let temp = TempDir::new().unwrap();
        let store = DocsStore::new(temp.path().join("docs"));
        let action = DownloadDocsAction {
            installer: FakeDocsInstaller::new(docs_json()),
            store: store.clone(),
            version: "0.14.2".to_string(),
            link: link(),
        };
        let mut events = Vec::new();

        let staging = action
            .run(&mut |event| events.push(event), &mut |_| {})
            .unwrap();

        assert!(staging.path().join("index.md").exists());
        assert!(staging.path().join("tutorial").join("writing.md").exists());
        assert!(store.resolve("0.14.2").unwrap().is_none());
        assert!(events.contains(&DownloadDocsEvent::Downloading {
            current: docs_json().len() as u64,
            total: docs_json().len() as u64,
        }));
        assert!(events.contains(&DownloadDocsEvent::Transforming));
    }
}
