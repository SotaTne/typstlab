use typstlab_proto::{Action, Collection, Model};
use thiserror::Error;
use std::fmt::Debug;

#[derive(Error, Debug)]
pub enum DiscoveryError {
    #[error("Entity not found for input: '{0}'")]
    NotFound(String),
    #[error("Failed to resolve entity for input '{input}': {source}")]
    ResolveFailed {
        input: String,
        #[source]
        source: crate::models::CollectionError,
    },
}

/// 曖昧な入力から実体(Model)を特定するアクション
pub struct DiscoveryAction<S> 
{
    pub scope: S,
    pub inputs: Vec<String>,
}

impl<T, S> Action<Vec<T>, (), (), DiscoveryError> for DiscoveryAction<S> 
where 
    T: Model + Debug,
    S: Collection<T, crate::models::CollectionError>
{
    fn run(
        self,
        _monitor: &mut dyn FnMut(()),
        _warning: &mut dyn FnMut(()),
    ) -> Result<Vec<T>, Vec<DiscoveryError>> {
        let mut results = Vec::new();
        let mut errors = Vec::new();

        for input in &self.inputs {
            match self.scope.resolve(input) {
                Ok(Some(item)) => results.push(item),
                Ok(None) => errors.push(DiscoveryError::NotFound(input.clone())),
                Err(source) => errors.push(DiscoveryError::ResolveFailed {
                    input: input.clone(),
                    source,
                }),
            }
        }

        if errors.is_empty() {
            Ok(results)
        } else {
            Err(errors)
        }
    }
}
