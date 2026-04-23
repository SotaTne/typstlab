use typstlab_proto::{Action, Creatable};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CreateError {
    #[error("Creation failed: {0}")]
    ExecutionError(String),
}

/// 実体の誕生中に発生するイベント
#[derive(Debug, Clone)]
pub enum CreateEvent {
    Initializing,
    Persisting,
    Completed,
}

/// 実体を誕生させるためのアクション
pub struct CreateAction<T: Creatable> {
    pub target: T,
    pub args: T::Args,
}

impl<T: Creatable> Action<T, CreateEvent, CreateError> for CreateAction<T> {
    fn run(mut self, monitor: &mut dyn FnMut(CreateEvent)) -> Result<T, Vec<CreateError>> {
        monitor(CreateEvent::Initializing);
        
        // 1. 引数を注入して初期化
        self.target.initialize(self.args);

        monitor(CreateEvent::Persisting);

        // 2. ファイルシステムへ固定
        self.target.persist()
            .map_err(|e| vec![CreateError::ExecutionError(e)])?;

        monitor(CreateEvent::Completed);

        Ok(self.target)
    }
}
