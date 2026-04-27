use crate::actions::resolve_typst::StoreError;
use crate::models::Docs;
use std::path::PathBuf;
use tempfile::TempDir;
use typstlab_base::persistence::Persistence;
use typstlab_proto::{Collection, Store};

/// Typst ドキュメント（KB）を管理する保管庫
pub struct DocsStore {
    pub root: PathBuf,
}

impl DocsStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn staging_root(&self) -> PathBuf {
        self.root.join(".tmp")
    }

    pub fn docs_path(&self, version: &str) -> PathBuf {
        self.root.join(version)
    }
}

typstlab_proto::impl_entity! {
    DocsStore {
        fn path(&self) -> PathBuf {
            self.root.clone()
        }
    }
}

impl Collection<Docs, StoreError> for DocsStore {
    fn list(&self) -> Result<Vec<Docs>, StoreError> {
        let mut list = Vec::new();
        if !self.root.exists() {
            return Ok(list);
        }

        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;
            if !entry.path().is_dir() {
                continue;
            }
            if let Some(version) = entry.file_name().to_str() {
                if version.starts_with('.') {
                    continue;
                }
                list.push(Docs { path: entry.path() });
            }
        }
        Ok(list)
    }

    fn resolve(&self, input: &str) -> Result<Option<Docs>, StoreError> {
        let path = self.docs_path(input);
        if path.exists() {
            Ok(Some(Docs { path }))
        } else {
            Ok(None)
        }
    }
}

impl Store<Docs, StoreError> for DocsStore {
    type Staging = TempDir;

    fn create_staging_area(&self, id: &str) -> Result<Self::Staging, StoreError> {
        let prefix = format!("staging-docs-{}-", id);
        Persistence::create_temp_dir(self.staging_root(), &prefix)
            .map_err(|e| StoreError::Io(std::io::Error::other(e)))
    }

    fn commit_staged(&self, id: &str, staging: Self::Staging) -> Result<Docs, StoreError> {
        let dest_path = self.docs_path(id);
        let staging_path = staging.path();

        Persistence::commit_directory(staging_path, &dest_path)
            .map_err(|e| StoreError::Io(std::io::Error::other(e)))?;

        Ok(Docs { path: dest_path })
    }
}

impl Clone for DocsStore {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use typstlab_proto::Store;

    #[test]
    fn test_docs_store_atomic_commit() {
        let temp = TempDir::new().unwrap();
        let store = DocsStore::new(temp.path().to_path_buf());
        let version = "0.14.2";

        let staging = store.create_staging_area(version).unwrap();
        let staging_path = staging.path().to_path_buf();

        // 疑似的なドキュメント生成 (Markdown群)
        std::fs::write(staging_path.join("overview.md"), b"# Overview").unwrap();

        let docs = store.commit_staged(version, staging).unwrap();

        assert!(docs.path.exists());
        assert!(docs.path.join("overview.md").exists());
        assert!(!staging_path.exists());
    }

    #[test]
    fn test_docs_store_remains_clean_on_failure() {
        let temp = TempDir::new().unwrap();
        let store = DocsStore::new(temp.path().to_path_buf());
        let version = "0.14.2";

        {
            let staging = store.create_staging_area(version).unwrap();
            std::fs::write(staging.path().join("overview.md"), b"# Overview").unwrap();
            // 何か書き込んでも commit しなければ消える
        }

        assert!(!store.docs_path(version).exists());
    }
}
