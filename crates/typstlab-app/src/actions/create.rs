use thiserror::Error;
use typstlab_proto::{Action, AppEvent, Creatable, EventScope, Loaded};

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

impl<T> Action for CreateAction<T>
where
    T: Creatable,
    T::Error: Send + Sync + 'static,
{
    type Output = Loaded<T, T::Config>;
    type Event = CreateEvent;
    type Warning = ();
    type Error = CreateError;

    fn run(
        self,
        monitor: &mut dyn FnMut(AppEvent<CreateEvent>),
        _warning: &mut dyn FnMut(Self::Warning),
    ) -> Result<Self::Output, Vec<Self::Error>> {
        let scope = EventScope::new("create");
        monitor(AppEvent::verbose(scope.clone(), CreateEvent::Initializing));

        let loaded = self
            .target
            .initialize(self.args)
            .map_err(|e| vec![CreateError::ExecutionError(Box::new(e))])?;

        monitor(AppEvent::verbose(scope.clone(), CreateEvent::Persisting));

        // 2. ファイルシステムへ固定
        T::persist(&loaded).map_err(|e| vec![CreateError::ExecutionError(Box::new(e))])?;

        monitor(AppEvent::verbose(scope, CreateEvent::Completed));

        Ok(loaded)
    }
}
