use crate::actions::resolve_typst::{ResolveEvent, StoreError};
use crate::models::{Docs, ManagedStore};
use typstlab_proto::Action;

pub struct ResolveDocsAction {
    pub store: ManagedStore,
    pub version: String,
}

impl Action<Docs, ResolveEvent, (), StoreError> for ResolveDocsAction {
    fn run(
        self,
        monitor: &mut dyn FnMut(ResolveEvent),
        _warning: &mut dyn FnMut(()),
    ) -> Result<Docs, Vec<StoreError>> {
        monitor(ResolveEvent::CheckingCache);

        let docs_path = self.store.root.join("docs").join(&self.version);

        if docs_path.exists() {
            monitor(ResolveEvent::CacheHit);
            monitor(ResolveEvent::Completed);
            return Ok(Docs { path: docs_path });
        }

        monitor(ResolveEvent::CacheMiss);

        Err(vec![StoreError::NotFound(format!(
            "Docs for version {}",
            self.version
        ))])
    }
}
