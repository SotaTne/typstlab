use std::path::Path;
use thiserror::Error;

use crate::install::{InstallProvider, ProgressReader};
use typstlab_proto::{Downloaded, Installer, SourceFormat};

#[derive(Debug, Error)]
pub enum DocsInstallError {
    #[error("Failed to access docs source: {0}")]
    SourceAccessFailed(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Unsupported format for DocsInstaller: {0:?}")]
    UnsupportedFormat(SourceFormat),

    #[error("I/O error during raw data streaming: {0}")]
    Io(#[from] std::io::Error),
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

    fn install<F>(
        &self,
        url: &str,
        format: SourceFormat,
        _dest: &Path,
        on_progress: F,
    ) -> Result<Downloaded, Self::Error>
    where
        F: FnMut(u64, u64) + Send + 'static,
    {
        if format != SourceFormat::Raw {
            return Err(DocsInstallError::UnsupportedFormat(format));
        }

        let (reader, total_size) = self
            .provider
            .fetch(url)
            .map_err(|e| DocsInstallError::SourceAccessFailed(Box::new(e)))?;

        let progress_reader = ProgressReader::new(reader, total_size, on_progress);

        Ok(Downloaded::Raw(Box::new(progress_reader)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::io::Read;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    // --- Mocks ---

    struct MockProvider {
        data: Result<Vec<u8>, std::io::Error>,
    }

    impl InstallProvider for MockProvider {
        type Error = DocsInstallError;
        fn fetch(&self, _url: &str) -> Result<(Box<dyn Read + Send>, u64), Self::Error> {
            match &self.data {
                Ok(bytes) => Ok((Box::new(Cursor::new(bytes.clone())), bytes.len() as u64)),
                Err(e) => Err(DocsInstallError::Io(std::io::Error::new(
                    e.kind(),
                    e.to_string(),
                ))),
            }
        }
    }

    // --- Tests ---

    #[test]
    fn test_docs_progress_linked_to_reading() {
        let data = vec![0u8; 1000];
        let total_size = data.len() as u64;
        let provider = MockProvider { data: Ok(data) };
        let installer = DocsInstaller::new(provider);
        let temp = TempDir::new().unwrap();

        let progress_history = Arc::new(Mutex::new(Vec::new()));
        let history_clone = progress_history.clone();

        let downloaded = installer
            .install("url", SourceFormat::Raw, temp.path(), move |curr, total| {
                let mut h = history_clone.lock().unwrap();
                h.push((curr, total));
            })
            .unwrap();

        if let Downloaded::Raw(mut reader) = downloaded {
            assert!(progress_history.lock().unwrap().is_empty());
            let mut buf = [0u8; 500];
            reader.read_exact(&mut buf).unwrap();
            assert_eq!(progress_history.lock().unwrap().last().unwrap().0, 500);
            let mut remaining = Vec::new();
            reader.read_to_end(&mut remaining).unwrap();
            assert_eq!(
                progress_history.lock().unwrap().last().unwrap().0,
                total_size
            );
        } else {
            panic!("Expected Downloaded::Raw");
        }
    }

    #[test]
    fn test_err_source_access_failed() {
        let provider = MockProvider {
            data: Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "api offline",
            )),
        };
        let installer = DocsInstaller::new(provider);
        let temp = TempDir::new().unwrap();
        let res = installer.install("url", SourceFormat::Raw, temp.path(), |_, _| {});
        assert!(matches!(res, Err(DocsInstallError::SourceAccessFailed(_))));
    }

    #[test]
    fn test_err_unsupported_format() {
        let provider = MockProvider { data: Ok(vec![]) };
        let installer = DocsInstaller::new(provider);
        let temp = TempDir::new().unwrap();
        let res = installer.install(
            "url",
            SourceFormat::TarXz {
                strip_components: 0,
            },
            temp.path(),
            |_, _| {},
        );
        assert!(matches!(res, Err(DocsInstallError::UnsupportedFormat(_))));
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
        let temp = TempDir::new().unwrap();
        let downloaded = installer
            .install("url", SourceFormat::Raw, temp.path(), |_, _| {})
            .unwrap();
        if let Downloaded::Raw(mut reader) = downloaded {
            let mut buf = Vec::new();
            let res = reader.read_to_end(&mut buf);
            assert!(res.is_err());
            assert_eq!(buf.len(), 3);
        }
    }
}
