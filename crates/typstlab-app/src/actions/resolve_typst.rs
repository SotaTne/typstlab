use crate::models::{Typst, TypstStore};
use thiserror::Error;
use typstlab_base::install::{InstallProvider, TypstInstaller};
use typstlab_base::link_resolver::ResolvedLink;
use typstlab_proto::{Action, AppEvent, Collection, EventScope, Installer, Store};

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Resource not found: {0}")]
    NotFound(String),
}

#[derive(Error, Debug)]
pub enum ResolveTypstError<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    #[error("store failed: {0}")]
    Store(#[from] StoreError),
    #[error("failed to install typst: {0}")]
    Install(#[source] E),
}

/// 解決プロセス中に発生するイベント
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveEvent {
    CheckingCache,
    CacheHit,
    CacheMiss,
    Completed,
}

pub struct ResolveTypstAction<P>
where
    P: InstallProvider,
{
    pub store: TypstStore,
    pub version: String,
    pub installer: TypstInstaller<P>,
    pub link: ResolvedLink,
}

impl<P> Action for ResolveTypstAction<P>
where
    P: InstallProvider,
{
    type Output = Typst;
    type Event = ResolveEvent;
    type Warning = ();
    type Error = ResolveTypstError<typstlab_base::install::TypstInstallError>;

    fn run(
        self,
        monitor: &mut dyn FnMut(AppEvent<ResolveEvent>),
        _warning: &mut dyn FnMut(Self::Warning),
    ) -> Result<Self::Output, Vec<Self::Error>> {
        self.run_inner(monitor).map_err(|error| vec![error])
    }
}

impl<P> ResolveTypstAction<P>
where
    P: InstallProvider,
{
    fn run_inner(
        self,
        monitor: &mut dyn FnMut(AppEvent<ResolveEvent>),
    ) -> Result<Typst, ResolveTypstError<typstlab_base::install::TypstInstallError>> {
        let scope = EventScope::labeled("resolve_typst", self.version.clone());
        monitor(AppEvent::verbose(
            scope.clone(),
            ResolveEvent::CheckingCache,
        ));

        if let Some(typst) = self.store.resolve(&self.version)? {
            monitor(AppEvent::verbose(scope.clone(), ResolveEvent::CacheHit));
            monitor(AppEvent::verbose(scope, ResolveEvent::Completed));
            return Ok(typst);
        }

        monitor(AppEvent::line(scope.clone(), ResolveEvent::CacheMiss));

        let installation = self
            .installer
            .install(&self.link.url, self.link.format, |_, _| {})
            .map_err(ResolveTypstError::Install)?;
        let typst = self.store.commit_staged(&self.version, installation)?;

        monitor(AppEvent::verbose(scope, ResolveEvent::Completed));
        Ok(typst)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Read};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use tar::{Builder as TarBuilder, Header as TarHeader};
    use tempfile::TempDir;
    use typstlab_base::link_resolver::ResolvedLink;
    use typstlab_proto::{Collection, SourceFormat, TYPST_BINARY_NAME};
    use xz2::write::XzEncoder;

    use super::*;

    #[derive(Debug, Error)]
    #[error("fake install failed")]
    struct FakeInstallError;

    struct FakeProvider {
        data: Vec<u8>,
        called: Arc<AtomicBool>,
    }

    impl FakeProvider {
        fn new(data: Vec<u8>) -> Self {
            Self {
                data,
                called: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    impl InstallProvider for FakeProvider {
        type Error = FakeInstallError;

        fn fetch(&self, url: &str) -> Result<(Box<dyn Read + Send>, u64), Self::Error> {
            assert_eq!(url, "https://example.com/typst.tar.xz");
            self.called.store(true, Ordering::SeqCst);
            Ok((
                Box::new(Cursor::new(self.data.clone())),
                self.data.len() as u64,
            ))
        }
    }

    fn create_tar_xz(entries: Vec<(&str, &[u8])>) -> Vec<u8> {
        let mut tar_buf = Vec::new();
        {
            let mut builder = TarBuilder::new(&mut tar_buf);
            for (path, content) in entries {
                let mut header = TarHeader::new_gnu();
                header.set_path(path).unwrap();
                header.set_size(content.len() as u64);
                header.set_cksum();
                builder.append(&header, content).unwrap();
            }
            builder.finish().unwrap();
        }

        let mut encoder = XzEncoder::new(Vec::new(), 6);
        std::io::copy(&mut Cursor::new(tar_buf), &mut encoder).unwrap();
        encoder.finish().unwrap()
    }

    fn typst_archive() -> Vec<u8> {
        create_tar_xz(vec![(
            &format!("typst-x86_64-apple-darwin/{}", TYPST_BINARY_NAME),
            b"typst",
        )])
    }

    fn empty_archive() -> Vec<u8> {
        create_tar_xz(Vec::new())
    }

    fn link() -> ResolvedLink {
        ResolvedLink {
            url: "https://example.com/typst.tar.xz".to_string(),
            format: SourceFormat::TarXz {
                strip_components: 1,
            },
        }
    }

    #[test]
    fn test_resolve_typst_installs_and_commits_when_missing() {
        let temp = TempDir::new().unwrap();
        let store = TypstStore::new(temp.path().join("typst"));
        let provider = FakeProvider::new(typst_archive());
        let called = provider.called.clone();
        let action = ResolveTypstAction {
            store: store.clone(),
            version: "0.14.2".to_string(),
            installer: TypstInstaller::new(provider),
            link: link(),
        };
        let mut events = Vec::new();

        let typst = action
            .run(&mut |event| events.push(event), &mut |_| {})
            .unwrap();

        assert!(called.load(Ordering::SeqCst));
        assert_eq!(typst.version, "0.14.2");
        assert!(typst.binary_path.exists());
        assert!(store.resolve("0.14.2").unwrap().is_some());
        assert!(
            events
                .iter()
                .any(|event| event.payload == ResolveEvent::CacheMiss)
        );
        assert!(
            events
                .iter()
                .any(|event| event.payload == ResolveEvent::Completed)
        );
    }

    #[test]
    fn test_resolve_typst_uses_cache_without_installing() {
        let temp = TempDir::new().unwrap();
        let store = TypstStore::new(temp.path().join("typst"));
        std::fs::create_dir_all(store.typst_path("0.14.2")).unwrap();
        std::fs::write(store.binary_path("0.14.2"), b"typst").unwrap();
        let provider = FakeProvider::new(empty_archive());
        let called = provider.called.clone();
        let action = ResolveTypstAction {
            store,
            version: "0.14.2".to_string(),
            installer: TypstInstaller::new(provider),
            link: link(),
        };
        let mut events = Vec::new();

        let typst = action
            .run(&mut |event| events.push(event), &mut |_| {})
            .unwrap();

        assert!(!called.load(Ordering::SeqCst));
        assert_eq!(typst.version, "0.14.2");
        assert!(
            events
                .iter()
                .any(|event| event.payload == ResolveEvent::CacheHit)
        );
        assert!(
            !events
                .iter()
                .any(|event| event.payload == ResolveEvent::CacheMiss)
        );
    }

    #[test]
    fn test_resolve_typst_errors_when_install_does_not_produce_binary() {
        let temp = TempDir::new().unwrap();
        let store = TypstStore::new(temp.path().join("typst"));
        let action = ResolveTypstAction {
            store,
            version: "0.14.2".to_string(),
            installer: TypstInstaller::new(FakeProvider::new(empty_archive())),
            link: link(),
        };

        let errors = match action.run(&mut |_| {}, &mut |_| {}) {
            Ok(_) => panic!("expected resolve failure"),
            Err(errors) => errors,
        };

        assert!(matches!(
            errors.as_slice(),
            [ResolveTypstError::Store(StoreError::NotFound(_))]
        ));
    }
}
