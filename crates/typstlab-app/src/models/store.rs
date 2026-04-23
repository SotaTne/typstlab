use crate::actions::resolve_typst::ResolveTypstAction;
use std::path::PathBuf;

pub struct ManagedStore {
    pub root: PathBuf,
}

typstlab_proto::impl_entity! {
    ManagedStore {
        fn path(&self) -> PathBuf {
            self.root.clone()
        }
    }
}

impl ManagedStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn typst_resolver(&self, version: &str) -> ResolveTypstAction {
        ResolveTypstAction {
            store_root: self.root.clone(),
            version: version.to_string(),
        }
    }

    pub fn docs_resolver(&self, version: &str) -> crate::actions::resolve_docs::ResolveDocsAction {
        crate::actions::resolve_docs::ResolveDocsAction {
            store: self.clone(),
            version: version.to_string(),
        }
    }

    pub fn typst_path(&self, version: &str) -> PathBuf {
        self.root.join("typst").join(version)
    }

    pub fn typst_binary_path(&self, version: &str) -> PathBuf {
        let base = self.typst_path(version);
        #[cfg(windows)]
        {
            base.join("typst.exe")
        }
        #[cfg(not(windows))]
        {
            base.join("typst")
        }
    }
}

impl Clone for ManagedStore {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
        }
    }
}
