use thiserror::Error;
use typstlab_proto::{Action, Creatable, Loaded};

#[derive(Error, Debug)]
pub enum CreateError {
    #[error("Creation failed: {0}")]
    ExecutionError(#[source] Box<dyn std::error::Error + Send + Sync>),
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

impl<T> Action<Loaded<T, T::Config>, CreateEvent, (), CreateError> for CreateAction<T>
where
    T: Creatable,
    T::Error: Send + Sync + 'static,
{
    fn run(
        self,
        monitor: &mut dyn FnMut(CreateEvent),
        _warning: &mut dyn FnMut(()),
    ) -> Result<Loaded<T, T::Config>, Vec<CreateError>> {
        monitor(CreateEvent::Initializing);

        let loaded = self
            .target
            .initialize(self.args)
            .map_err(|e| vec![CreateError::ExecutionError(Box::new(e))])?;

        monitor(CreateEvent::Persisting);

        // 2. ファイルシステムへ固定
        T::persist(&loaded).map_err(|e| vec![CreateError::ExecutionError(Box::new(e))])?;

        monitor(CreateEvent::Completed);

        Ok(loaded)
    }
}
