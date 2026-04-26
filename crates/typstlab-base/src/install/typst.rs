use std::io::{self, copy};
use std::path::{Path, PathBuf};
use thiserror::Error;

// 外部ライブラリの型を明示的にインポート
use xz2::read::XzDecoder;
use tar::Archive as TarArchive;
use zip::ZipArchive;

use typstlab_proto::{Downloaded, Installer, SourceFormat};
use crate::install::{InstallProvider, ProgressReader};

#[derive(Debug, Error)]
pub enum TypstInstallError {
    #[error("Failed to access source: {0}")]
    SourceAccessFailed(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("XZ decompression failed: {0}")]
    XzDecompressionFailed(#[source] io::Error),

    #[error("TAR extraction failed: {0}")]
    TarExtractionFailed(#[source] io::Error),

    #[error("ZIP extraction failed: {0}")]
    ZipExtractionFailed(#[from] zip::result::ZipError),

    #[error("Failed to create temporary file for ZIP extraction: {0}")]
    ZipStagingFailed(#[source] io::Error),

    #[error("Archive path '{path}' is too shallow to strip {required} components")]
    PathStripFailed { path: PathBuf, required: usize },

    #[error("Failed to create directory '{path}': {source}")]
    DirectoryCreationFailed { path: PathBuf, #[source] source: io::Error },

    #[error("Failed to write extracted file '{path}': {source}")]
    FileWriteFailed { path: PathBuf, #[source] source: io::Error },

    #[error("Unsupported format for TypstInstaller: {0:?}")]
    UnsupportedFormat(SourceFormat),

    #[error("General I/O error: {0}")]
    Io(#[from] io::Error),
}

pub struct TypstInstaller<P: InstallProvider> {
    provider: P,
}

impl<P: InstallProvider> TypstInstaller<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl<P: InstallProvider> Installer for TypstInstaller<P> {
    type Error = TypstInstallError;

    fn install<F>(
        &self,
        url: &str,
        format: SourceFormat,
        dest: &Path,
        on_progress: F,
    ) -> Result<Downloaded, Self::Error>
    where
        F: FnMut(u64, u64) + Send + 'static,
    {
        let (reader, total_size) = self.provider.fetch(url)
            .map_err(|e| TypstInstallError::SourceAccessFailed(Box::new(e)))?;

        let mut progress_reader = ProgressReader::new(reader, total_size, on_progress);

        match format {
            SourceFormat::TarXz { strip_components } => {
                let xz = XzDecoder::new(progress_reader);
                let mut archive = TarArchive::new(xz);
                
                for entry in archive.entries().map_err(TypstInstallError::TarExtractionFailed)? {
                    let mut entry = entry.map_err(TypstInstallError::TarExtractionFailed)?;
                    let path = entry.path().map_err(TypstInstallError::TarExtractionFailed)?.to_path_buf();
                    
                    if let Some(stripped_path) = strip_path(&path, strip_components) {
                        let out_path = dest.join(stripped_path);
                        if let Some(parent) = out_path.parent() {
                            std::fs::create_dir_all(parent).map_err(|e| TypstInstallError::DirectoryCreationFailed {
                                path: parent.to_path_buf(),
                                source: e,
                            })?;
                        }
                        entry.unpack(&out_path).map_err(|e| TypstInstallError::FileWriteFailed {
                            path: out_path,
                            source: e,
                        })?;
                    } else if strip_components > 0 {
                        return Err(TypstInstallError::PathStripFailed {
                            path,
                            required: strip_components,
                        });
                    }
                }
                Ok(Downloaded::Archive(dest.to_path_buf()))
            }
            SourceFormat::Zip { strip_components } => {
                let mut tmp_file = tempfile::tempfile().map_err(TypstInstallError::ZipStagingFailed)?;
                copy(&mut progress_reader, &mut tmp_file).map_err(TypstInstallError::Io)?;
                
                let mut archive = ZipArchive::new(tmp_file)?;
                
                for i in 0..archive.len() {
                    let mut file = archive.by_index(i)?;
                    let path = PathBuf::from(file.name());
                    
                    if let Some(stripped_path) = strip_path(&path, strip_components) {
                        let out_path = dest.join(stripped_path);
                        if file.is_dir() {
                            std::fs::create_dir_all(&out_path).map_err(|e| TypstInstallError::DirectoryCreationFailed {
                                path: out_path,
                                source: e,
                            })?;
                        } else {
                            if let Some(parent) = out_path.parent() {
                                std::fs::create_dir_all(parent).map_err(|e| TypstInstallError::DirectoryCreationFailed {
                                    path: parent.to_path_buf(),
                                    source: e,
                                })?;
                            }
                            let mut out_file = std::fs::File::create(&out_path).map_err(|e| TypstInstallError::FileWriteFailed {
                                path: out_path.clone(),
                                source: e,
                            })?;
                            copy(&mut file, &mut out_file).map_err(|e| TypstInstallError::FileWriteFailed {
                                path: out_path,
                                source: e,
                            })?;
                        }
                    }
                }
                Ok(Downloaded::Archive(dest.to_path_buf()))
            }
            SourceFormat::Raw => Err(TypstInstallError::UnsupportedFormat(SourceFormat::Raw)),
        }
    }
}

fn strip_path(path: &Path, count: usize) -> Option<PathBuf> {
    let mut components = path.components();
    for _ in 0..count {
        components.next()?;
    }
    let stripped = components.as_path();
    if stripped.as_os_str().is_empty() {
        None
    } else {
        Some(stripped.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read, Write};
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;
    
    use xz2::write::XzEncoder;
    use tar::Builder as TarBuilder;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

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
        data: Result<Vec<u8>, TypstInstallError>,
        chunk_size: usize,
    }

    impl InstallProvider for MockProvider {
        type Error = TypstInstallError;
        fn fetch(&self, _url: &str) -> Result<(Box<dyn Read + Send>, u64), Self::Error> {
            match &self.data {
                Ok(bytes) => {
                    let size = bytes.len() as u64;
                    Ok((Box::new(ForcedChunkedReader { inner: Cursor::new(bytes.clone()), chunk_size: self.chunk_size }), size))
                }
                Err(e) => Err(TypstInstallError::Io(io::Error::new(io::ErrorKind::Other, e.to_string()))),
            }
        }
    }

    fn create_tar_xz(entries: Vec<(&str, &[u8])>) -> Vec<u8> {
        let mut tar_buf = Vec::new();
        {
            let mut builder = TarBuilder::new(&mut tar_buf);
            for (path, content) in entries {
                let mut header = tar::Header::new_gnu();
                header.set_path(path).unwrap();
                header.set_size(content.len() as u64);
                header.set_cksum();
                builder.append(&header, content).unwrap();
            }
            builder.finish().unwrap();
        }
        let mut xz_buf = Vec::new();
        let mut encoder = XzEncoder::new(&mut xz_buf, 6);
        encoder.write_all(&tar_buf).unwrap();
        encoder.finish().unwrap();
        xz_buf
    }

    fn create_zip(entries: Vec<(&str, &[u8])>) -> Vec<u8> {
        let mut zip_buf = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut zip_buf));
            for (path, content) in entries {
                zip.start_file(path, SimpleFileOptions::default()).unwrap();
                zip.write_all(content).unwrap();
            }
            zip.finish().unwrap();
        }
        zip_buf
    }

    // --- Tests ---

    #[test]
    fn test_progress_incremental_tar_xz() {
        let xz_data = create_tar_xz(vec![("test.txt", &[0u8; 4096])]);
        let total_size = xz_data.len() as u64;
        let provider = MockProvider { data: Ok(xz_data), chunk_size: 64 };
        let installer = TypstInstaller::new(provider);
        let temp = TempDir::new().unwrap();

        let progress_history = Arc::new(Mutex::new(Vec::new()));
        let history_clone = progress_history.clone();

        installer.install("url", SourceFormat::TarXz { strip_components: 0 }, temp.path(), move |curr, total| {
            let mut h = history_clone.lock().unwrap();
            h.push((curr, total));
        }).unwrap();

        let h = progress_history.lock().unwrap();
        assert!(h.len() > 1, "Progress should be reported multiple times");
        let (last_curr, last_total) = h.last().unwrap();
        assert!(*last_curr > 0);
        assert_eq!(*last_total, total_size);
    }

    #[test]
    fn test_progress_incremental_zip() {
        let zip_data = create_zip(vec![("f.txt", &[0u8; 1024])]);
        let total_size = zip_data.len() as u64;
        let provider = MockProvider { data: Ok(zip_data), chunk_size: 64 };
        let installer = TypstInstaller::new(provider);
        let temp = TempDir::new().unwrap();

        let progress_history = Arc::new(Mutex::new(Vec::new()));
        let history_clone = progress_history.clone();

        installer.install("url", SourceFormat::Zip { strip_components: 0 }, temp.path(), move |curr, _| {
            let mut h = history_clone.lock().unwrap();
            if h.last().map(|&last| last < curr).unwrap_or(true) {
                h.push(curr);
            }
        }).unwrap();

        let h = progress_history.lock().unwrap();
        assert!(h.len() > 1);
        assert_eq!(*h.last().unwrap(), total_size);
    }

    #[test]
    fn test_err_source_access_failed_wrapping() {
        let provider = MockProvider { 
            data: Err(TypstInstallError::Io(io::Error::new(io::ErrorKind::ConnectionRefused, "offline"))),
            chunk_size: 1024
        };
        let installer = TypstInstaller::new(provider);
        let temp = TempDir::new().unwrap();
        let res = installer.install("url", SourceFormat::TarXz { strip_components: 0 }, temp.path(), |_,_| {});
        match res {
            Err(TypstInstallError::SourceAccessFailed(e)) => assert!(e.to_string().contains("offline")),
            _ => panic!("Expected SourceAccessFailed"),
        }
    }

    #[test]
    fn test_err_xz_decode_failed() {
        let provider = MockProvider { data: Ok(vec![0xDE, 0xAD, 0xBE, 0xEF]), chunk_size: 1024 }; 
        let installer = TypstInstaller::new(provider);
        let temp = TempDir::new().unwrap();
        let res = installer.install("url", SourceFormat::TarXz { strip_components: 0 }, temp.path(), |_,_| {});
        assert!(matches!(res, Err(TypstInstallError::TarExtractionFailed(_))));
    }

    #[test]
    fn test_err_zip_extraction_failed() {
        let provider = MockProvider { data: Ok(vec![0x50, 0x4B, 0x03, 0x04]), chunk_size: 1024 }; 
        let installer = TypstInstaller::new(provider);
        let temp = TempDir::new().unwrap();
        let res = installer.install("url", SourceFormat::Zip { strip_components: 0 }, temp.path(), |_,_| {});
        assert!(matches!(res, Err(TypstInstallError::ZipExtractionFailed(_))));
    }

    #[test]
    fn test_err_path_strip_too_deep() {
        let xz_data = create_tar_xz(vec![("root.txt", b"content")]);
        let provider = MockProvider { data: Ok(xz_data), chunk_size: 1024 };
        let installer = TypstInstaller::new(provider);
        let temp = TempDir::new().unwrap();
        let res = installer.install("url", SourceFormat::TarXz { strip_components: 1 }, temp.path(), |_,_| {});
        assert!(matches!(res, Err(TypstInstallError::PathStripFailed { required: 1, .. })));
    }

    #[test]
    fn test_err_directory_conflict() {
        let xz_data = create_tar_xz(vec![("dir/file", b"")]);
        let provider = MockProvider { data: Ok(xz_data), chunk_size: 1024 };
        let installer = TypstInstaller::new(provider);
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("dir"), b"").unwrap();
        let res = installer.install("url", SourceFormat::TarXz { strip_components: 0 }, temp.path(), |_,_| {});
        assert!(matches!(res, Err(TypstInstallError::DirectoryCreationFailed { .. })));
    }

    #[test]
    fn test_err_file_write_failed() {
        let xz_data = create_tar_xz(vec![("readonly.txt", b"hi")]);
        let provider = MockProvider { data: Ok(xz_data), chunk_size: 1024 };
        let installer = TypstInstaller::new(provider);
        let temp = TempDir::new().unwrap();
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(temp.path()).unwrap().permissions();
            perms.set_mode(0o444); 
            std::fs::set_permissions(temp.path(), perms).unwrap();
        }
        let res = installer.install("url", SourceFormat::TarXz { strip_components: 0 }, temp.path(), |_,_| {});
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(temp.path()).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(temp.path(), perms).unwrap();
        }
        assert!(matches!(res, Err(TypstInstallError::FileWriteFailed { .. })));
    }

    #[test]
    fn test_err_unsupported_format() {
        let provider = MockProvider { data: Ok(vec![]), chunk_size: 1024 };
        let installer = TypstInstaller::new(provider);
        let temp = TempDir::new().unwrap();
        let res = installer.install("url", SourceFormat::Raw, temp.path(), |_,_| {});
        assert!(matches!(res, Err(TypstInstallError::UnsupportedFormat(_))));
    }
}
