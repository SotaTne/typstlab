use std::path::{Path, PathBuf};

use thiserror::Error;

pub mod binary;
pub mod docs;

pub use binary::{BinaryStoreKey, StoredBinary, ToolchainBinaryStore};
pub use docs::{DocsStoreKey, StoredDocsTree, ToolchainDocsStore};

#[derive(Debug, Error)]
pub enum ToolchainStoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("destination already exists: {path}")]
    DestinationAlreadyExists { path: PathBuf },
}

pub(crate) fn create_staging(
    root: &Path,
    prefix: &str,
) -> Result<tempfile::TempDir, ToolchainStoreError> {
    let staging_root = root.join(".tmp");
    std::fs::create_dir_all(&staging_root)?;
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(staging_root)
        .map_err(ToolchainStoreError::Io)
}

pub(crate) fn commit_directory(
    staging: &tempfile::TempDir,
    destination: &Path,
) -> Result<(), ToolchainStoreError> {
    // staging の所有権は Store::commit が持つ。
    // helper は rename だけを担当し、失敗時の cleanup は TempDir の Drop に任せる。
    if destination.exists() {
        return Err(ToolchainStoreError::DestinationAlreadyExists {
            path: destination.to_path_buf(),
        });
    }

    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::rename(staging.path(), destination)?;
    Ok(())
}
