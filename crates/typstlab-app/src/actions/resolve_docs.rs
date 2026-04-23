use crate::actions::resolve_typst::{ResolveEvent, StoreError};
use crate::models::Docs;
use std::path::PathBuf;
use typstlab_proto::Action;

pub struct ResolveDocsAction {
    pub store_root: PathBuf,
    pub version: String,
}

impl Action<Docs, ResolveEvent, (), StoreError> for ResolveDocsAction {
    fn run(
        self,
        monitor: &mut dyn FnMut(ResolveEvent),
        _warning: &mut dyn FnMut(()),
    ) -> Result<Docs, Vec<StoreError>> {
        monitor(ResolveEvent::CheckingCache);

        let docs_path = self.store_root.join("docs").join(&self.version);

        if docs_path.exists() {
            monitor(ResolveEvent::CacheHit);
            monitor(ResolveEvent::Completed);
            return Ok(Docs {
                version: self.version.clone(),
                project_root: PathBuf::new(),
            });
        }

        monitor(ResolveEvent::CacheMiss);

        Err(vec![StoreError::NotFound(format!(
            "Docs for version {}",
            self.version
        ))])
    }
}
