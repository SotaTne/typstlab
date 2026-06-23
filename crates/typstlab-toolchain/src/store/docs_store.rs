use std::path::PathBuf;

use typstlab_proto::path::HasRoot;
use typstlab_proto::{Store, StoreIndex};

use super::{ToolchainStoreError, commit_directory, create_staging};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocsStoreKey {
    pub docs_id: String,
    pub version: String,
}

impl DocsStoreKey {
    pub fn new(docs_id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            docs_id: docs_id.into(),
            version: version.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredDocsTree {
    pub root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ToolchainDocsStore {
    root: PathBuf,
}

impl ToolchainDocsStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn path_for(&self, key: &DocsStoreKey) -> PathBuf {
        self.root.join(&key.docs_id).join(&key.version)
    }
}

impl HasRoot for ToolchainDocsStore {
    fn root(&self) -> PathBuf {
        self.root.clone()
    }
}

impl Store for ToolchainDocsStore {
    type Item = StoredDocsTree;
    type Key = DocsStoreKey;
    type Error = ToolchainStoreError;
    type Staging = tempfile::TempDir;

    fn resolve(&self, key: &Self::Key) -> Result<Option<Self::Item>, Self::Error> {
        let root = self.path_for(key);
        Ok(root.exists().then_some(StoredDocsTree { root }))
    }

    fn stage(&self, key: &Self::Key) -> Result<Self::Staging, Self::Error> {
        create_staging(
            &self.root,
            &format!("docs-{}-{}-", key.docs_id, key.version),
        )
    }

    fn commit(&self, key: &Self::Key, staging: Self::Staging) -> Result<Self::Item, Self::Error> {
        let destination = self.path_for(key);
        commit_directory(staging.path(), &destination)?;
        Ok(StoredDocsTree { root: destination })
    }
}

impl StoreIndex for ToolchainDocsStore {
    fn list(&self) -> Result<Vec<Self::Item>, Self::Error> {
        let mut docs = Vec::new();
        if !self.root.exists() {
            return Ok(docs);
        }

        for docs_id in std::fs::read_dir(&self.root)? {
            let docs_id = docs_id?;
            if !docs_id.file_type()?.is_dir() || is_hidden(&docs_id.file_name().to_string_lossy()) {
                continue;
            }
            for version in std::fs::read_dir(docs_id.path())? {
                let version = version?;
                if version.file_type()?.is_dir()
                    && !is_hidden(&version.file_name().to_string_lossy())
                {
                    docs.push(StoredDocsTree {
                        root: version.path(),
                    });
                }
            }
        }

        Ok(docs)
    }
}

fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docs_store_commits_staged_directory_by_docs_id_and_version() {
        let temp = tempfile::TempDir::new().unwrap();
        let store = ToolchainDocsStore::new(temp.path().join("docs"));
        let key = DocsStoreKey::new("typst-docs", "0.14.2");

        let staging = store.stage(&key).unwrap();
        std::fs::write(staging.path().join("index.md"), b"# Docs").unwrap();

        let stored = store.commit(&key, staging).unwrap();

        assert_eq!(
            stored.root,
            temp.path().join("docs").join("typst-docs").join("0.14.2")
        );
        assert!(stored.root.join("index.md").exists());
        assert_eq!(store.resolve(&key).unwrap(), Some(stored));
    }
}
