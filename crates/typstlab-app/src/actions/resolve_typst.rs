use crate::models::Typst;
use std::path::PathBuf;
use thiserror::Error;
use typstlab_proto::Action;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Resource not found: {0}")]
    NotFound(String),
}

/// 解決プロセス中に発生するイベント
#[derive(Debug, Clone)]
pub enum ResolveEvent {
    CheckingCache,
    CacheHit,
    CacheMiss,
    Completed,
}

pub struct ResolveTypstAction {
    pub store_root: PathBuf,
    pub version: String,
}

impl Action<Typst, ResolveEvent, StoreError> for ResolveTypstAction {
    fn run(self, monitor: &mut dyn FnMut(ResolveEvent)) -> Result<Typst, Vec<StoreError>> {
        monitor(ResolveEvent::CheckingCache);

        #[cfg(not(windows))]
        const BINARY_NAME: &str = "typst";

        #[cfg(windows)]
        const BINARY_NAME: &str = "typst.exe";

        let binary_path = self
            .store_root
            .join("typst")
            .join(&self.version)
            .join(BINARY_NAME);

        if binary_path.exists() {
            monitor(ResolveEvent::CacheHit);
            monitor(ResolveEvent::Completed);
            return Ok(Typst {
                version: self.version.clone(),
                binary_path,
            });
        }

        monitor(ResolveEvent::CacheMiss);

        Err(vec![StoreError::NotFound(self.version.clone())])
    }
}
