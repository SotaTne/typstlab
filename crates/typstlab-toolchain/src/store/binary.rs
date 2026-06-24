use std::path::PathBuf;

use typstlab_proto::path::HasRoot;
use typstlab_proto::{Store, StoreEntryAddress, StoreIndex};

use crate::{Arch, Os, Platform};

use super::{ToolchainStoreError, commit_directory, create_staging};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryStoreKey {
    pub tool_id: String,
    pub version: String,
    pub platform: Platform,
}

impl BinaryStoreKey {
    pub fn new(tool_id: impl Into<String>, version: impl Into<String>, platform: Platform) -> Self {
        Self {
            tool_id: tool_id.into(),
            version: version.into(),
            platform,
        }
    }
}

impl StoreEntryAddress for BinaryStoreKey {
    fn relative_path(&self) -> PathBuf {
        PathBuf::from(&self.tool_id)
            .join(&self.version)
            .join(platform_path_segment(self.platform))
    }

    fn staging_prefix(&self) -> String {
        format!(
            "binary-{}-{}-{}-",
            self.tool_id,
            self.version,
            platform_path_segment(self.platform)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredBinary {
    /// tool/version/platform ごとに commit された install root。
    /// executable_relative_path はこの path からの相対 path として解決する。
    pub install_root: PathBuf,
}

impl HasRoot for StoredBinary {
    fn root(&self) -> PathBuf {
        self.install_root.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ToolchainBinaryStore {
    root: PathBuf,
}

impl ToolchainBinaryStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl HasRoot for ToolchainBinaryStore {
    fn root(&self) -> PathBuf {
        self.root.clone()
    }
}

impl Store for ToolchainBinaryStore {
    type Item = StoredBinary;
    type Key = BinaryStoreKey;
    type Error = ToolchainStoreError;
    type Staging = tempfile::TempDir;

    fn resolve(&self, key: &Self::Key) -> Result<Option<Self::Item>, Self::Error> {
        let root = self.path_for(key);
        Ok(root.exists().then_some(StoredBinary { install_root: root }))
    }

    fn stage(&self, key: &Self::Key) -> Result<Self::Staging, Self::Error> {
        create_staging(&self.root, &key.staging_prefix())
    }

    fn commit(&self, key: &Self::Key, staging: Self::Staging) -> Result<Self::Item, Self::Error> {
        let destination = self.path_for(key);
        commit_directory(&staging, &destination)?;
        Ok(StoredBinary {
            install_root: destination,
        })
    }
}

impl StoreIndex for ToolchainBinaryStore {
    fn list(&self) -> Result<Vec<Self::Item>, Self::Error> {
        let mut binaries = Vec::new();
        if !self.root.exists() {
            return Ok(binaries);
        }

        for tool in std::fs::read_dir(&self.root)? {
            let tool = tool?;
            if !tool.file_type()?.is_dir() || is_hidden(&tool.file_name().to_string_lossy()) {
                continue;
            }
            for version in std::fs::read_dir(tool.path())? {
                let version = version?;
                if !version.file_type()?.is_dir()
                    || is_hidden(&version.file_name().to_string_lossy())
                {
                    continue;
                }
                for platform in std::fs::read_dir(version.path())? {
                    let platform = platform?;
                    if platform.file_type()?.is_dir()
                        && !is_hidden(&platform.file_name().to_string_lossy())
                    {
                        binaries.push(StoredBinary {
                            install_root: platform.path(),
                        });
                    }
                }
            }
        }

        Ok(binaries)
    }
}

fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

fn platform_path_segment(platform: Platform) -> &'static str {
    match (platform.os, platform.arch) {
        (Os::MacOS, Arch::X86_64) => "x86_64-apple-darwin",
        (Os::MacOS, Arch::Aarch64) => "aarch64-apple-darwin",
        (Os::Linux, Arch::X86_64) => "x86_64-unknown-linux-musl",
        (Os::Linux, Arch::Aarch64) => "aarch64-unknown-linux-musl",
        (Os::Linux, Arch::Riscv64) => "riscv64gc-unknown-linux-gnu",
        (Os::Windows, Arch::X86_64) => "x86_64-pc-windows-msvc",
        (Os::Windows, Arch::Aarch64) => "aarch64-pc-windows-msvc",
        (Os::Windows, Arch::Riscv64) | (Os::MacOS, Arch::Riscv64) => "unsupported",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_store_commits_staged_directory_by_tool_version_and_platform() {
        let temp = tempfile::TempDir::new().unwrap();
        let store = ToolchainBinaryStore::new(temp.path().join("binary"));
        let key = BinaryStoreKey::new(
            "typst",
            "0.14.2",
            Platform {
                os: Os::MacOS,
                arch: Arch::Aarch64,
            },
        );

        let staging = store.stage(&key).unwrap();
        std::fs::write(staging.path().join("typst"), b"binary").unwrap();

        let stored = store.commit(&key, staging).unwrap();

        assert_eq!(
            stored.root(),
            temp.path()
                .join("binary")
                .join("typst")
                .join("0.14.2")
                .join("aarch64-apple-darwin")
        );
        assert!(stored.install_root.join("typst").exists());
        assert_eq!(store.resolve(&key).unwrap(), Some(stored));
    }
}
