use typstlab_proto::Action;
use crate::models::{Paper, PaperScope};
use typstlab_proto::Collection;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum DiscoveryError {
    #[error("Paper not found for input: '{0}'")]
    NotFound(String),
}

/// 曖昧な入力から実体を特定するアクション
pub struct DiscoveryAction {
    pub scope: PaperScope,
    pub inputs: Vec<String>,
}

impl Action<Vec<Paper>, (), DiscoveryError> for DiscoveryAction {
    fn run(self, _monitor: &mut dyn FnMut(())) -> Result<Vec<Paper>, Vec<DiscoveryError>> {
        let mut papers = Vec::new();
        let mut errors = Vec::new();

        for input in &self.inputs {
            if let Some(paper) = self.scope.resolve(input) {
                papers.push(paper);
            } else {
                errors.push(DiscoveryError::NotFound(input.clone()));
            }
        }

        if errors.is_empty() {
            Ok(papers)
        } else {
            Err(errors)
        }
    }
}
