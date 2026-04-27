use crate::actions::resolve_typst::StoreError;
use crate::models::Typst;
use std::path::PathBuf;
use tempfile::TempDir;
use typstlab_base::persistence::Persistence;
use typstlab_proto::{Collection, Store};

/// Typst バイナリを管理する保管庫
pub struct TypstStore {
    pub root: PathBuf,
}

impl TypstStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn staging_root(&self) -> PathBuf {
        self.root.join(".tmp")
    }

    pub fn typst_path(&self, version: &str) -> PathBuf {
        self.root.join(version)
    }

    pub fn binary_path(&self, version: &str) -> PathBuf {
        let base = self.typst_path(version);
        if cfg!(windows) {
            base.join("typst.exe")
        } else {
            base.join("typst")
        }
    }
}

typstlab_proto::impl_entity! {
    TypstStore {
        fn path(&self) -> PathBuf {
            self.root.clone()
        }
    }
}

impl Collection<Typst, StoreError> for TypstStore {
    fn list(&self) -> Result<Vec<Typst>, StoreError> {
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
                let bin = self.binary_path(version);
                if bin.exists() {
                    list.push(Typst::new(version.to_string(), bin));
                }
            }
        }
        Ok(list)
    }

    fn resolve(&self, input: &str) -> Result<Option<Typst>, StoreError> {
        let bin = self.binary_path(input);
        if bin.exists() {
            Ok(Some(Typst::new(input.to_string(), bin)))
        } else {
            Ok(None)
        }
    }
}

impl Store<Typst, StoreError> for TypstStore {
    type Staging = TempDir;

    fn create_staging_area(&self, id: &str) -> Result<Self::Staging, StoreError> {
        let prefix = format!("staging-{}-", id);
        Persistence::create_temp_dir(self.staging_root(), &prefix)
            .map_err(|e| StoreError::Io(std::io::Error::other(e)))
    }

    fn commit_staged(&self, id: &str, staging: Self::Staging) -> Result<Typst, StoreError> {
        let dest_path = self.typst_path(id);
        let staging_path = staging.path();

        Persistence::commit_directory(staging_path, &dest_path)
            .map_err(|e| StoreError::Io(std::io::Error::other(e)))?;

        let bin = self.binary_path(id);
        if !bin.exists() {
            return Err(StoreError::NotFound(format!(
                "Binary not found after commit for {}",
                id
            )));
        }

        Ok(Typst::new(id.to_string(), bin))
    }
}

impl Clone for TypstStore {
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
    fn test_typst_store_atomic_commit() {
        let temp = TempDir::new().unwrap();
        let store = TypstStore::new(temp.path().to_path_buf());
        let version = "0.14.2";

        // 1. ステージングエリアの作成
        let staging = store.create_staging_area(version).unwrap();
        let staging_path = staging.path().to_path_buf();
        assert!(staging_path.exists());

        // 2. 作業場にバイナリを模したファイルを配置 (モデルから名前を取得)
        let bin_filename = store
            .binary_path(version)
            .file_name()
            .unwrap()
            .to_os_string();
        std::fs::write(staging_path.join(bin_filename), b"dummy binary").unwrap();

        // 3. コミット実行
        let typst = store.commit_staged(version, staging).unwrap();

        // 4. 正式な領土に実体が生まれ、作業場が消えていることを確認
        assert_eq!(typst.version, version);
        assert!(store.typst_path(version).exists());
        assert!(store.binary_path(version).exists());
        assert!(!staging_path.exists());
    }

    #[test]
    fn test_typst_store_remains_clean_on_drop_without_commit() {
        let temp = TempDir::new().unwrap();
        let store = TypstStore::new(temp.path().to_path_buf());
        let version = "0.14.2";

        {
            let staging = store.create_staging_area(version).unwrap();
            let staging_path = staging.path().to_path_buf();
            std::fs::write(staging_path.join("dummy.txt"), b"some data").unwrap();
            // ここで commit せずにスコープを抜ける (drop される)
        }

        // 正式な領土は汚染されていない
        assert!(!store.typst_path(version).exists());
    }

    #[test]
    fn test_typst_store_resolve_only_returns_valid_installs() {
        let temp = TempDir::new().unwrap();
        let store = TypstStore::new(temp.path().to_path_buf());
        let version = "0.14.2";

        // ディレクトリだけあってもバイナリがない場合は None
        std::fs::create_dir_all(store.typst_path(version)).unwrap();
        let resolved = store.resolve(version).unwrap();
        assert!(resolved.is_none());

        // バイナリを置けば Some
        let bin_path = store.binary_path(version);
        std::fs::write(bin_path, b"dummy").unwrap();
        let resolved = store.resolve(version).unwrap();
        assert!(resolved.is_some());
    }
}
