use std::io::{self, Read, copy};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use thiserror::Error;

// 外部ライブラリの型を明示的にインポート
use tar::Archive as TarArchive;
use xz2::read::XzDecoder;
use zip::ZipArchive;

use crate::install::{InstallProvider, ProgressReader};
use typstlab_proto::{Installer, SourceFormat};

// 基盤共通ユーティリティをインポート
use crate::path::{is_path_safe, strip_path};

#[derive(Debug, Error)]
pub enum TypstInstallError {
    #[error("Failed to access source: {0}")]
    SourceAccessFailed(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("XZ decompression failed: {0}")]
    XzDecompressionFailed(#[source] io::Error),

    #[error("TAR extraction failed: {0}")]
    TarExtractionFailed(#[source] io::Error),

    #[error("Failed to create temporary directory for Typst install: {0}")]
    StagingCreationFailed(#[source] io::Error),

    #[error("ZIP extraction failed: {0}")]
    ZipExtractionFailed(#[from] zip::result::ZipError),

    #[error("Failed to create temporary file for ZIP extraction: {0}")]
    ZipStagingFailed(#[source] io::Error),

    #[error("Archive path '{path}' is too shallow to strip {required} components")]
    PathStripFailed { path: PathBuf, required: usize },

    #[error("Failed to create directory '{path}': {source}")]
    DirectoryCreationFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Failed to write extracted file '{path}': {source}")]
    FileWriteFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Unsupported format for TypstInstaller: {0:?}")]
    UnsupportedFormat(SourceFormat),

    #[error("General I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Malicious path detected in archive: {0}")]
    SecurityError(String),
}

pub struct TypstInstaller<P: InstallProvider> {
    provider: P,
}

struct XzReadTracker<R: Read> {
    inner: XzDecoder<R>,
    failed: Arc<AtomicBool>,
}

impl<R: Read> XzReadTracker<R> {
    fn new(reader: R) -> (Self, Arc<AtomicBool>) {
        let failed = Arc::new(AtomicBool::new(false));
        (
            Self {
                inner: XzDecoder::new(reader),
                failed: failed.clone(),
            },
            failed,
        )
    }
}

impl<R: Read> Read for XzReadTracker<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.inner.read(buf) {
            Ok(n) => Ok(n),
            Err(e) => {
                self.failed.store(true, Ordering::SeqCst);
                Err(e)
            }
        }
    }
}

fn tar_or_xz_error(error: io::Error, xz_failed: &AtomicBool) -> TypstInstallError {
    if xz_failed.load(Ordering::SeqCst) {
        TypstInstallError::XzDecompressionFailed(error)
    } else {
        TypstInstallError::TarExtractionFailed(error)
    }
}

impl<P: InstallProvider> TypstInstaller<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl<P: InstallProvider> Installer for TypstInstaller<P> {
    type Error = TypstInstallError;
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
        self.install_with_factories(
            url,
            format,
            on_progress,
            tempfile::TempDir::new,
            tempfile::tempfile,
        )
    }
}

impl<P: InstallProvider> TypstInstaller<P> {
    fn install_with_factories<F, C, S>(
        &self,
        url: &str,
        format: SourceFormat,
        on_progress: F,
        create_tempdir: C,
        create_zip_staging: S,
    ) -> Result<tempfile::TempDir, TypstInstallError>
    where
        F: FnMut(u64, u64) + Send + 'static,
        C: FnOnce() -> io::Result<tempfile::TempDir>,
        S: FnOnce() -> io::Result<std::fs::File>,
    {
        if format == SourceFormat::Raw {
            return Err(TypstInstallError::UnsupportedFormat(format));
        }

        let installation = create_tempdir().map_err(TypstInstallError::StagingCreationFailed)?;
        self.install_into(url, format, installation, on_progress, create_zip_staging)
    }

    fn install_into<F, S>(
        &self,
        url: &str,
        format: SourceFormat,
        installation: tempfile::TempDir,
        on_progress: F,
        create_zip_staging: S,
    ) -> Result<tempfile::TempDir, TypstInstallError>
    where
        F: FnMut(u64, u64) + Send + 'static,
        S: FnOnce() -> io::Result<std::fs::File>,
    {
        let (reader, total_size) = self
            .provider
            .fetch(url)
            .map_err(|e| TypstInstallError::SourceAccessFailed(Box::new(e)))?;

        let mut progress_reader = ProgressReader::new(reader, total_size, on_progress);
        let dest = installation.path();

        match format {
            SourceFormat::TarXz { strip_components } => {
                let (xz, xz_failed) = XzReadTracker::new(progress_reader);
                let mut archive = TarArchive::new(xz);

                for entry in archive
                    .entries()
                    .map_err(|e| tar_or_xz_error(e, &xz_failed))?
                {
                    let mut entry = entry.map_err(|e| tar_or_xz_error(e, &xz_failed))?;
                    let path = entry
                        .path()
                        .map_err(|e| tar_or_xz_error(e, &xz_failed))?
                        .to_path_buf();

                    let stripped = strip_path(&path, strip_components);

                    match stripped {
                        Some(stripped_path) => {
                            if !is_path_safe(&stripped_path) {
                                return Err(TypstInstallError::SecurityError(
                                    path.display().to_string(),
                                ));
                            }

                            if let Some(link_name) = entry
                                .link_name()
                                .map_err(|e| tar_or_xz_error(e, &xz_failed))?
                                .filter(|link_name| !is_path_safe(link_name))
                            {
                                return Err(TypstInstallError::SecurityError(format!(
                                    "{} -> {}",
                                    path.display(),
                                    link_name.display()
                                )));
                            }

                            let out_path = dest.join(&stripped_path);
                            if let Some(parent) = out_path.parent() {
                                std::fs::create_dir_all(parent).map_err(|e| {
                                    TypstInstallError::DirectoryCreationFailed {
                                        path: parent.to_path_buf(),
                                        source: e,
                                    }
                                })?;
                            }
                            entry.unpack(&out_path).map_err(|e| {
                                TypstInstallError::FileWriteFailed {
                                    path: out_path,
                                    source: e,
                                }
                            })?;
                        }
                        None if strip_components > 0 => {
                            if entry.header().entry_type().is_dir() {
                                continue;
                            }
                            return Err(TypstInstallError::PathStripFailed {
                                path,
                                required: strip_components,
                            });
                        }
                        None => {
                            if !is_path_safe(&path) {
                                return Err(TypstInstallError::SecurityError(
                                    path.display().to_string(),
                                ));
                            }
                            entry
                                .unpack_in(dest)
                                .map_err(|e| tar_or_xz_error(e, &xz_failed))?;
                        }
                    }
                }
                Ok(installation)
            }
            SourceFormat::Zip { strip_components } => {
                let mut tmp_file =
                    create_zip_staging().map_err(TypstInstallError::ZipStagingFailed)?;
                copy(&mut progress_reader, &mut tmp_file).map_err(TypstInstallError::Io)?;

                let mut archive = ZipArchive::new(tmp_file)?;

                for i in 0..archive.len() {
                    let mut file = archive.by_index(i)?;

                    let safe_name = file
                        .enclosed_name()
                        .ok_or_else(|| TypstInstallError::SecurityError(file.name().to_string()))?
                        .to_path_buf();

                    let stripped = strip_path(&safe_name, strip_components);

                    match stripped {
                        Some(stripped_path) => {
                            if !is_path_safe(&stripped_path) {
                                return Err(TypstInstallError::SecurityError(
                                    safe_name.display().to_string(),
                                ));
                            }

                            let out_path = dest.join(&stripped_path);
                            if file.is_dir() {
                                std::fs::create_dir_all(&out_path).map_err(|e| {
                                    TypstInstallError::DirectoryCreationFailed {
                                        path: out_path,
                                        source: e,
                                    }
                                })?;
                            } else {
                                if let Some(parent) = out_path.parent() {
                                    std::fs::create_dir_all(parent).map_err(|e| {
                                        TypstInstallError::DirectoryCreationFailed {
                                            path: parent.to_path_buf(),
                                            source: e,
                                        }
                                    })?;
                                }
                                let mut out_file =
                                    std::fs::File::create(&out_path).map_err(|e| {
                                        TypstInstallError::FileWriteFailed {
                                            path: out_path.clone(),
                                            source: e,
                                        }
                                    })?;
                                copy(&mut file, &mut out_file).map_err(|e| {
                                    TypstInstallError::FileWriteFailed {
                                        path: out_path,
                                        source: e,
                                    }
                                })?;
                            }
                        }
                        None if strip_components > 0 => {
                            if file.is_dir() {
                                continue;
                            }
                            return Err(TypstInstallError::PathStripFailed {
                                path: safe_name,
                                required: strip_components,
                            });
                        }
                        None => {}
                    }
                }
                Ok(installation)
            }
            SourceFormat::Raw => Err(TypstInstallError::UnsupportedFormat(SourceFormat::Raw)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read, Write};
    use std::sync::{Arc, Mutex};

    use tar::Builder as TarBuilder;
    use tar::Header as TarHeader;
    use xz2::write::XzEncoder;
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
                    Ok((
                        Box::new(ForcedChunkedReader {
                            inner: Cursor::new(bytes.clone()),
                            chunk_size: self.chunk_size,
                        }),
                        size,
                    ))
                }
                Err(e) => Err(TypstInstallError::Io(io::Error::other(e.to_string()))),
            }
        }
    }

    /// TAR + XZ データを生成する。
    ///
    /// `force_raw_path` が `true` の場合、OS の tar クレートが path を検証する前に
    /// GNU ヘッダの name フィールドへ生バイトで書き込む（Windows でもルートパスを埋め込める）。
    fn create_tar_xz_raw(entries: Vec<(&str, &[u8], Option<&str>)>) -> Vec<u8> {
        create_tar_xz_with_opts(entries, false)
    }

    fn create_tar_xz_with_opts(
        entries: Vec<(&str, &[u8], Option<&str>)>,
        force_raw_path: bool,
    ) -> Vec<u8> {
        let mut tar_buf = Vec::new();
        {
            let mut builder = TarBuilder::new(&mut tar_buf);
            for (path, content, link) in entries {
                let mut header = TarHeader::new_gnu();
                if force_raw_path {
                    // Windows の tar クレートが set_path で弾く前に
                    // GNU ヘッダの name フィールド（100 バイト）へ直接書き込む。
                    let gnu = header.as_gnu_mut().unwrap();
                    let bytes = path.as_bytes();
                    let len = bytes.len().min(gnu.name.len() - 1);
                    gnu.name[..len].copy_from_slice(&bytes[..len]);
                    // null 終端はデフォルトで 0 になっている
                } else {
                    header.set_path(path).unwrap();
                }
                header.set_size(content.len() as u64);
                if let Some(l) = link {
                    header.set_entry_type(tar::EntryType::Symlink);
                    header.set_link_name(l).unwrap();
                }
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

    fn create_xz_raw_bytes(bytes: &[u8]) -> Vec<u8> {
        let mut xz_buf = Vec::new();
        let mut encoder = XzEncoder::new(&mut xz_buf, 6);
        encoder.write_all(bytes).unwrap();
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

    // --- 1. Progress ---

    #[test]
    fn test_progress_incremental_tar_xz() {
        let xz_data = create_tar_xz_raw(vec![("large.txt", &[0u8; 1024], None)]);
        let total_size = xz_data.len() as u64;
        let provider = MockProvider {
            data: Ok(xz_data),
            chunk_size: 1,
        };
        let installer = TypstInstaller::new(provider);
        let progress_history = Arc::new(Mutex::new(Vec::new()));
        let history_clone = progress_history.clone();
        let installation = installer
            .install(
                "url",
                SourceFormat::TarXz {
                    strip_components: 0,
                },
                move |curr, total| {
                    let mut h = history_clone.lock().unwrap();
                    if h.last()
                        .map(|&(last_curr, _)| last_curr < curr)
                        .unwrap_or(true)
                    {
                        h.push((curr, total));
                    }
                },
            )
            .unwrap();
        assert!(installation.path().join("large.txt").exists());
        let h = progress_history.lock().unwrap();
        assert!(h.len() > 1);
        assert_eq!(h.last().unwrap().1, total_size);
    }

    #[test]
    fn test_progress_incremental_zip() {
        let zip_data = create_zip(vec![("f.txt", &[0u8; 1024])]);
        let total_size = zip_data.len() as u64;
        let provider = MockProvider {
            data: Ok(zip_data),
            chunk_size: 1,
        };
        let installer = TypstInstaller::new(provider);
        let progress_history = Arc::new(Mutex::new(Vec::new()));
        let history_clone = progress_history.clone();
        let installation = installer
            .install(
                "url",
                SourceFormat::Zip {
                    strip_components: 0,
                },
                move |curr, total| {
                    let mut h = history_clone.lock().unwrap();
                    if h.last()
                        .map(|&(last_curr, _)| last_curr < curr)
                        .unwrap_or(true)
                    {
                        h.push((curr, total));
                    }
                },
            )
            .unwrap();
        assert!(installation.path().join("f.txt").exists());
        let h = progress_history.lock().unwrap();
        assert!(h.len() > 1);
        assert_eq!(h.last().unwrap().1, total_size);
    }

    // --- 2. Security ---

    #[test]
    fn test_err_security_rooted_path_tar() {
        // Windows の tar クレートは set_path でバックスラッシュ始まりのパスを弾くため、
        // GNU ヘッダへ直接書き込んで typstlab 側のセキュリティチェックを検証する。
        let xz_data = create_tar_xz_with_opts(vec![("\\Windows\\evil.txt", b"evil", None)], true);
        let provider = MockProvider {
            data: Ok(xz_data),
            chunk_size: 1024,
        };
        let installer = TypstInstaller::new(provider);
        let res = installer.install(
            "url",
            SourceFormat::TarXz {
                strip_components: 0,
            },
            |_, _| {},
        );
        assert!(matches!(res, Err(TypstInstallError::SecurityError(_))));
    }

    #[test]
    fn test_err_security_malicious_link_tar() {
        let xz_data = create_tar_xz_raw(vec![("link.txt", b"", Some("/etc/passwd"))]);
        let provider = MockProvider {
            data: Ok(xz_data),
            chunk_size: 1024,
        };
        let installer = TypstInstaller::new(provider);
        let res = installer.install(
            "url",
            SourceFormat::TarXz {
                strip_components: 0,
            },
            |_, _| {},
        );
        assert!(matches!(res, Err(TypstInstallError::SecurityError(_))));
    }

    #[test]
    fn test_err_security_traversal_zip_deep() {
        let zip_data = create_zip(vec![("a/../../evil.txt", b"evil")]);
        let provider = MockProvider {
            data: Ok(zip_data),
            chunk_size: 1024,
        };
        let installer = TypstInstaller::new(provider);
        let res = installer.install(
            "url",
            SourceFormat::Zip {
                strip_components: 0,
            },
            |_, _| {},
        );
        assert!(matches!(res, Err(TypstInstallError::SecurityError(_))));
    }

    // --- 3. FS Logic ---

    #[test]
    fn test_strip_components_handles_directory_entries() {
        let mut tar_buf = Vec::new();
        {
            let mut builder = TarBuilder::new(&mut tar_buf);
            let mut header = TarHeader::new_gnu();
            header.set_path("v1/").unwrap();
            header.set_entry_type(tar::EntryType::Directory);
            header.set_size(0);
            header.set_cksum();
            builder.append(&header, &[][..]).unwrap();
            builder.finish().unwrap();
        }
        let mut xz_buf = Vec::new();
        let mut encoder = XzEncoder::new(&mut xz_buf, 6);
        encoder.write_all(&tar_buf).unwrap();
        encoder.finish().unwrap();

        let provider = MockProvider {
            data: Ok(xz_buf),
            chunk_size: 1024,
        };
        let installer = TypstInstaller::new(provider);
        let res = installer.install(
            "url",
            SourceFormat::TarXz {
                strip_components: 1,
            },
            |_, _| {},
        );
        assert!(res.is_ok());
    }

    #[test]
    fn test_err_path_strip_too_deep() {
        let xz_data = create_tar_xz_raw(vec![("file.txt", b"hi", None)]);
        let provider = MockProvider {
            data: Ok(xz_data),
            chunk_size: 1024,
        };
        let installer = TypstInstaller::new(provider);
        let res = installer.install(
            "url",
            SourceFormat::TarXz {
                strip_components: 1,
            },
            |_, _| {},
        );
        assert!(matches!(
            res,
            Err(TypstInstallError::PathStripFailed { required: 1, .. })
        ));
    }

    #[test]
    fn test_err_directory_conflict() {
        let zip_data = create_zip(vec![("blocked/file.txt", b"hi")]);
        let provider = MockProvider {
            data: Ok(zip_data),
            chunk_size: 1024,
        };
        let installer = TypstInstaller::new(provider);
        let installation = tempfile::TempDir::new().unwrap();
        std::fs::write(installation.path().join("blocked"), b"not a directory").unwrap();
        let res = installer.install_into(
            "url",
            SourceFormat::Zip {
                strip_components: 0,
            },
            installation,
            |_, _| {},
            tempfile::tempfile,
        );
        assert!(matches!(
            res,
            Err(TypstInstallError::DirectoryCreationFailed { .. })
        ));
    }

    #[test]
    fn test_err_file_write_failed() {
        let zip_data = create_zip(vec![("blocked.txt", b"hi")]);
        let provider = MockProvider {
            data: Ok(zip_data),
            chunk_size: 1024,
        };
        let installer = TypstInstaller::new(provider);
        let installation = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(installation.path().join("blocked.txt")).unwrap();
        let res = installer.install_into(
            "url",
            SourceFormat::Zip {
                strip_components: 0,
            },
            installation,
            |_, _| {},
            tempfile::tempfile,
        );
        assert!(matches!(
            res,
            Err(TypstInstallError::FileWriteFailed { .. })
        ));
    }

    #[test]
    fn test_err_zip_staging_failed() {
        let zip_data = create_zip(vec![("f.txt", b"hi")]);
        let provider = MockProvider {
            data: Ok(zip_data),
            chunk_size: 1024,
        };
        let installer = TypstInstaller::new(provider);
        let res = installer.install_with_factories(
            "url",
            SourceFormat::Zip {
                strip_components: 0,
            },
            |_, _| {},
            tempfile::TempDir::new,
            || Err(io::Error::other("staging failed")),
        );
        assert!(matches!(res, Err(TypstInstallError::ZipStagingFailed(_))));
    }

    #[test]
    fn test_err_staging_creation_failed() {
        let zip_data = create_zip(vec![("f.txt", b"hi")]);
        let provider = MockProvider {
            data: Ok(zip_data),
            chunk_size: 1024,
        };
        let installer = TypstInstaller::new(provider);
        let res = installer.install_with_factories(
            "url",
            SourceFormat::Zip {
                strip_components: 0,
            },
            |_, _| {},
            || Err(io::Error::other("tempdir failed")),
            tempfile::tempfile,
        );
        assert!(matches!(
            res,
            Err(TypstInstallError::StagingCreationFailed(_))
        ));
    }

    // --- 4. Error Handling ---

    #[test]
    fn test_err_source_access_failed_wrapping() {
        let provider = MockProvider {
            data: Err(TypstInstallError::Io(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "offline",
            ))),
            chunk_size: 1024,
        };
        let installer = TypstInstaller::new(provider);
        let res = installer.install(
            "url",
            SourceFormat::TarXz {
                strip_components: 0,
            },
            |_, _| {},
        );
        assert!(matches!(res, Err(TypstInstallError::SourceAccessFailed(_))));
    }

    #[test]
    fn test_err_xz_decode_failed() {
        let provider = MockProvider {
            data: Ok(vec![0x00, 0x01]),
            chunk_size: 1024,
        };
        let installer = TypstInstaller::new(provider);
        let res = installer.install(
            "url",
            SourceFormat::TarXz {
                strip_components: 0,
            },
            |_, _| {},
        );
        assert!(matches!(
            res,
            Err(TypstInstallError::XzDecompressionFailed(_))
        ));
    }

    #[test]
    fn test_err_tar_extraction_failed() {
        let xz_data = create_xz_raw_bytes(&[1u8; 512]);
        let provider = MockProvider {
            data: Ok(xz_data),
            chunk_size: 1024,
        };
        let installer = TypstInstaller::new(provider);
        let res = installer.install(
            "url",
            SourceFormat::TarXz {
                strip_components: 0,
            },
            |_, _| {},
        );
        assert!(matches!(
            res,
            Err(TypstInstallError::TarExtractionFailed(_))
        ));
    }

    #[test]
    fn test_err_io_during_zip_download() {
        struct FaultyReader {
            emitted: bool,
        }

        impl Read for FaultyReader {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                if self.emitted {
                    return Err(io::Error::other("download interrupted"));
                }
                self.emitted = true;
                buf[0] = 0;
                Ok(1)
            }
        }

        struct FaultyProvider;

        impl InstallProvider for FaultyProvider {
            type Error = TypstInstallError;

            fn fetch(&self, _url: &str) -> Result<(Box<dyn Read + Send>, u64), Self::Error> {
                Ok((Box::new(FaultyReader { emitted: false }), 2))
            }
        }

        let installer = TypstInstaller::new(FaultyProvider);
        let res = installer.install(
            "url",
            SourceFormat::Zip {
                strip_components: 0,
            },
            |_, _| {},
        );
        assert!(matches!(res, Err(TypstInstallError::Io(_))));
    }

    #[test]
    fn test_err_zip_extraction_failed() {
        let provider = MockProvider {
            data: Ok(vec![0x50, 0x4B, 0x03, 0x04]),
            chunk_size: 1024,
        };
        let installer = TypstInstaller::new(provider);
        let res = installer.install(
            "url",
            SourceFormat::Zip {
                strip_components: 0,
            },
            |_, _| {},
        );
        assert!(matches!(
            res,
            Err(TypstInstallError::ZipExtractionFailed(_))
        ));
    }

    #[test]
    fn test_err_unsupported_format() {
        let provider = MockProvider {
            data: Ok(vec![]),
            chunk_size: 1024,
        };
        let installer = TypstInstaller::new(provider);
        let res = installer.install("url", SourceFormat::Raw, |_, _| {});
        assert!(matches!(res, Err(TypstInstallError::UnsupportedFormat(_))));
    }

    #[test]
    fn test_mock_provider_uses_typst_install_error() {
        fn assert_error_type<T: InstallProvider<Error = TypstInstallError>>(_: &T) {}

        let provider = MockProvider {
            data: Ok(vec![]),
            chunk_size: 1024,
        };
        assert_error_type(&provider);
    }
}
