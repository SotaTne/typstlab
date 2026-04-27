use std::io::{self, copy};
use std::path::PathBuf;
use thiserror::Error;

use crate::install::{InstallProvider, ProgressReader};
use typstlab_proto::{Installer, SourceFormat};

pub const RAW_DOCS_FILENAME: &str = "downloaded.raw";

#[derive(Debug, Error)]
pub enum DocsInstallError {
    #[error("Failed to access docs source: {0}")]
    SourceAccessFailed(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Failed to create temporary directory for docs install: {0}")]
    StagingCreationFailed(#[source] io::Error),

    #[error("Failed to create raw docs file '{path}': {source}")]
    RawFileCreationFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Failed to write raw docs file '{path}': {source}")]
    RawFileWriteFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Unsupported format for DocsInstaller: {0:?}")]
    UnsupportedFormat(SourceFormat),

    #[error("General I/O error during docs install: {0}")]
    Io(#[from] io::Error),
}

pub struct DocsInstaller<P: InstallProvider> {
    provider: P,
}

impl<P: InstallProvider> DocsInstaller<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl<P: InstallProvider> Installer for DocsInstaller<P> {
    type Error = DocsInstallError;
    type Installation = tempfile::TempDir;

    fn install<F>(
        &self,
        url: &str,
        format: SourceFormat,
        on_progress: F,
    ) -> Result<Self::Installation, Self::Error>
    where
        F: FnMut(u64, u64) + Send + 'static,
    {
        self.install_with_tempdir_factory(url, format, on_progress, tempfile::TempDir::new)
    }
}

impl<P: InstallProvider> DocsInstaller<P> {
    fn install_with_tempdir_factory<F, C>(
        &self,
        url: &str,
        format: SourceFormat,
        on_progress: F,
        create_tempdir: C,
    ) -> Result<tempfile::TempDir, DocsInstallError>
    where
        F: FnMut(u64, u64) + Send + 'static,
        C: FnOnce() -> io::Result<tempfile::TempDir>,
    {
        if format != SourceFormat::Raw {
            return Err(DocsInstallError::UnsupportedFormat(format));
        }

        let installation = create_tempdir().map_err(DocsInstallError::StagingCreationFailed)?;
        self.install_into(url, installation, on_progress)
    }

    fn install_into<F>(
        &self,
        url: &str,
        installation: tempfile::TempDir,
        on_progress: F,
    ) -> Result<tempfile::TempDir, DocsInstallError>
    where
        F: FnMut(u64, u64) + Send + 'static,
    {
        let raw_path = installation.path().join(RAW_DOCS_FILENAME);
        let (reader, total_size) = self
            .provider
            .fetch(url)
            .map_err(|e| DocsInstallError::SourceAccessFailed(Box::new(e)))?;

        let mut progress_reader = ProgressReader::new(reader, total_size, on_progress);
        let mut raw_file = std::fs::File::create(&raw_path).map_err(|e| {
            DocsInstallError::RawFileCreationFailed {
                path: raw_path.clone(),
                source: e,
            }
        })?;

        copy(&mut progress_reader, &mut raw_file).map_err(|e| {
            DocsInstallError::RawFileWriteFailed {
                path: raw_path,
                source: e,
            }
        })?;

        Ok(installation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::io::Read;
    use std::sync::{Arc, Mutex};

    // --- Mocks ---

    struct ForcedChunkedReader {
        inner: Cursor<Vec<u8>>,
        chunk_size: usize,
    }

    impl Read for ForcedChunkedReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let limit = std::cmp::min(buf.len(), self.chunk_size);
            self.inner.read(&mut buf[..limit])
        }
    }

    struct MockProvider {
        data: Result<Vec<u8>, DocsInstallError>,
        chunk_size: usize,
    }

    impl InstallProvider for MockProvider {
        type Error = DocsInstallError;
        fn fetch(&self, _url: &str) -> Result<(Box<dyn Read + Send>, u64), Self::Error> {
            match &self.data {
                Ok(bytes) => Ok((
                    Box::new(ForcedChunkedReader {
                        inner: Cursor::new(bytes.clone()),
                        chunk_size: self.chunk_size,
                    }),
                    bytes.len() as u64,
                )),
                Err(e) => Err(DocsInstallError::Io(io::Error::other(e.to_string()))),
            }
        }
    }

    // --- Tests ---

    #[test]
    fn test_progress_incremental_raw_docs() {
        let data = vec![0u8; 1000];
        let total_size = data.len() as u64;
        let provider = MockProvider {
            data: Ok(data),
            chunk_size: 64,
        };
        let installer = DocsInstaller::new(provider);
        let progress_history = Arc::new(Mutex::new(Vec::new()));
        let history_clone = progress_history.clone();

        let installation = installer
            .install("url", SourceFormat::Raw, move |curr, total| {
                let mut h = history_clone.lock().unwrap();
                if h.last()
                    .map(|&(last_curr, _)| last_curr < curr)
                    .unwrap_or(true)
                {
                    h.push((curr, total));
                }
            })
            .unwrap();

        let raw_path = installation.path().join(RAW_DOCS_FILENAME);
        assert_eq!(std::fs::read(raw_path).unwrap(), vec![0u8; 1000]);
        let h = progress_history.lock().unwrap();
        assert!(h.len() > 1, "progress should be reported incrementally");
        assert_eq!(h.last().unwrap().0, total_size);
        assert_eq!(h.last().unwrap().1, total_size);
    }

    #[test]
    fn test_err_source_access_failed() {
        let provider = MockProvider {
            data: Err(DocsInstallError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "api offline",
            ))),
            chunk_size: 64,
        };
        let installer = DocsInstaller::new(provider);
        let res = installer.install("url", SourceFormat::Raw, |_, _| {});
        assert!(matches!(res, Err(DocsInstallError::SourceAccessFailed(_))));
    }

    #[test]
    fn test_err_unsupported_format() {
        let provider = MockProvider {
            data: Ok(vec![]),
            chunk_size: 64,
        };
        let installer = DocsInstaller::new(provider);
        let res = installer.install(
            "url",
            SourceFormat::TarXz {
                strip_components: 0,
            },
            |_, _| {},
        );
        assert!(matches!(res, Err(DocsInstallError::UnsupportedFormat(_))));
    }

    #[test]
    fn test_err_staging_creation_failed() {
        let provider = MockProvider {
            data: Ok(vec![]),
            chunk_size: 64,
        };
        let installer = DocsInstaller::new(provider);
        let res = installer.install_with_tempdir_factory(
            "url",
            SourceFormat::Raw,
            |_, _| {},
            || Err(io::Error::new(io::ErrorKind::PermissionDenied, "no temp")),
        );
        assert!(matches!(
            res,
            Err(DocsInstallError::StagingCreationFailed(_))
        ));
    }

    #[test]
    fn test_err_raw_file_creation_failed() {
        let provider = MockProvider {
            data: Ok(vec![1, 2, 3]),
            chunk_size: 64,
        };
        let installer = DocsInstaller::new(provider);
        let installation = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(installation.path().join(RAW_DOCS_FILENAME)).unwrap();

        let res = installer.install_into("url", installation, |_, _| {});
        assert!(matches!(
            res,
            Err(DocsInstallError::RawFileCreationFailed { .. })
        ));
    }

    #[test]
    fn test_err_io_propagation_mid_stream() {
        struct FaultyReader {
            count: usize,
        }
        impl Read for FaultyReader {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                if self.count > 0 {
                    self.count -= 1;
                    buf[0] = 0;
                    Ok(1)
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "Stream Broken",
                    ))
                }
            }
        }

        struct FaultyProvider;
        impl InstallProvider for FaultyProvider {
            type Error = DocsInstallError;
            fn fetch(&self, _url: &str) -> Result<(Box<dyn Read + Send>, u64), Self::Error> {
                Ok((Box::new(FaultyReader { count: 3 }), 100))
            }
        }

        let installer = DocsInstaller::new(FaultyProvider);
        let res = installer.install("url", SourceFormat::Raw, |_, _| {});
        assert!(matches!(
            res,
            Err(DocsInstallError::RawFileWriteFailed { .. })
        ));
    }

    #[test]
    fn test_mock_provider_uses_docs_install_error() {
        fn assert_error_type<P: InstallProvider<Error = DocsInstallError>>(_: &P) {}

        let provider = MockProvider {
            data: Ok(vec![]),
            chunk_size: 64,
        };
        assert_error_type(&provider);
    }
}
