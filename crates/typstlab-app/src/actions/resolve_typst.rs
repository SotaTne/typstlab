use crate::models::Typst;
use std::path::PathBuf;
use thiserror::Error;
use typstlab_proto::{Action, AppEvent, EventScope, TYPST_BINARY_NAME};

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

impl Action for ResolveTypstAction {
    type Output = Typst;
    type Event = ResolveEvent;
    type Warning = ();
    type Error = StoreError;

    fn run(
        self,
        monitor: &mut dyn FnMut(AppEvent<ResolveEvent>),
        _warning: &mut dyn FnMut(Self::Warning),
    ) -> Result<Self::Output, Vec<Self::Error>> {
        let scope = EventScope::labeled("resolve_typst", self.version.clone());
        monitor(AppEvent::verbose(
            scope.clone(),
            ResolveEvent::CheckingCache,
        ));

        let binary_path = self.store_root.join(&self.version).join(TYPST_BINARY_NAME);

        if binary_path.exists() {
            monitor(AppEvent::verbose(scope.clone(), ResolveEvent::CacheHit));
            monitor(AppEvent::verbose(scope, ResolveEvent::Completed));
            return Ok(Typst {
                version: self.version.clone(),
                binary_path,
            });
        }

        monitor(AppEvent::line(scope, ResolveEvent::CacheMiss));

        Err(vec![StoreError::NotFound(self.version.clone())])
    }
}
