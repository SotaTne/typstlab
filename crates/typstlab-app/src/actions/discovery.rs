use typstlab_proto::Action;
use crate::models::{Paper, PaperScope};
use typstlab_proto::Collection;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiscoveryError {
    #[error("Paper not found for input: '{0}'")]
    NotFound(String),
    #[error("Failed to resolve paper for input '{input}': {source}")]
    ResolveFailed {
        input: String,
        #[source]
        source: crate::models::CollectionError,
    },
}

/// 曖昧な入力から実体を特定するアクション
pub struct DiscoveryAction {
    pub scope: PaperScope,
    pub inputs: Vec<String>,
}

impl Action<Vec<Paper>, (), (), DiscoveryError> for DiscoveryAction {
    fn run(
        self,
        _monitor: &mut dyn FnMut(()),
        _warning: &mut dyn FnMut(()),
    ) -> Result<Vec<Paper>, Vec<DiscoveryError>> {
        let mut papers = Vec::new();
        let mut errors = Vec::new();

        for input in &self.inputs {
            match self.scope.resolve(input) {
                Ok(Some(paper)) => papers.push(paper),
                Ok(None) => errors.push(DiscoveryError::NotFound(input.clone())),
                Err(source) => errors.push(DiscoveryError::ResolveFailed {
                    input: input.clone(),
                    source,
                }),
            }
        }

        if errors.is_empty() {
            Ok(papers)
        } else {
            Err(errors)
        }
    }
}
