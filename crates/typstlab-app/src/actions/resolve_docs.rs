use crate::actions::resolve_typst::{ResolveEvent, StoreError};
use crate::models::{Docs, DocsStore};
use typstlab_proto::{Action, Collection};

pub struct ResolveDocsAction {
    pub store: DocsStore,
    pub version: String,
}

impl Action<Docs, ResolveEvent, (), StoreError> for ResolveDocsAction {
    fn run(
        self,
        monitor: &mut dyn FnMut(ResolveEvent),
        _warning: &mut dyn FnMut(()),
    ) -> Result<Docs, Vec<StoreError>> {
        monitor(ResolveEvent::CheckingCache);

        match self.store.resolve(&self.version) {
            Ok(Some(docs)) => {
                monitor(ResolveEvent::CacheHit);
                monitor(ResolveEvent::Completed);
                Ok(docs)
            }
            Ok(None) => {
                monitor(ResolveEvent::CacheMiss);
                Err(vec![StoreError::NotFound(format!(
                    "Docs for version {}",
                    self.version
                ))])
            }
            Err(e) => Err(vec![e]),
        }
    }
}
