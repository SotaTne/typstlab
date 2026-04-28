use std::fmt::Debug;
use std::marker::PhantomData;
use thiserror::Error;
use typstlab_proto::{Action, AppEvent, Collection, Model};

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
pub struct DiscoveryAction<S, T> {
    pub scope: S,
    pub inputs: Vec<String>,
    model: PhantomData<T>,
}

impl<S, T> DiscoveryAction<S, T> {
    pub fn new(scope: S, inputs: Vec<String>) -> Self {
        Self {
            scope,
            inputs,
            model: PhantomData,
        }
    }
}

impl<T, S> Action for DiscoveryAction<S, T>
where
    T: Model + Debug + 'static,
    S: Collection<T, crate::models::CollectionError>,
{
    type Output = Vec<T>;
    type Event = ();
    type Warning = ();
    type Error = DiscoveryError;

    fn run(
        self,
        _monitor: &mut dyn FnMut(AppEvent<()>),
        _warning: &mut dyn FnMut(Self::Warning),
    ) -> Result<Self::Output, Vec<Self::Error>> {
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
