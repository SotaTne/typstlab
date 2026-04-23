use std::path::PathBuf;
use typstlab_proto::Entity;
use crate::actions::resolve_typst::ResolveTypstAction;
use crate::actions::resolve_docs::ResolveDocsAction;

pub struct ManagedStore {
    pub root: PathBuf,
}

impl ManagedStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Typst 解決用のアクションを生成
    pub fn typst_resolver(&self, version: &str) -> ResolveTypstAction {
        ResolveTypstAction {
            store_root: self.root.clone(),
            version: version.to_string(),
        }
    }

    /// Docs 解決用のアクションを生成
    pub fn docs_resolver(&self, version: &str) -> ResolveDocsAction {
        ResolveDocsAction {
            store_root: self.root.clone(),
            version: version.to_string(),
        }
    }
}

impl Entity for ManagedStore {
    fn path(&self) -> PathBuf {
        self.root.clone()
    }
}
