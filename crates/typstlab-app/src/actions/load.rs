use std::fmt::Debug;
use typstlab_proto::{Action, AppEvent, EventScope, Loadable, Loaded};

/// 実体のロード（復元）中に発生するイベント
#[derive(Debug, Clone)]
pub enum LoadEvent {
    Started,
    Validating,
    Completed,
}

/// 実体をファイルシステムからロードするための汎用アクション
pub struct LoadAction<T: Loadable> {
    pub target: T,
}

impl<T: Loadable> Action for LoadAction<T> {
    type Output = Loaded<T, T::Config>;
    type Event = LoadEvent;
    type Warning = ();
    type Error = T::Error;

    fn run(
        self,
        monitor: &mut dyn FnMut(AppEvent<LoadEvent>),
        _warning: &mut dyn FnMut(Self::Warning),
    ) -> Result<Self::Output, Vec<Self::Error>> {
        let scope = EventScope::new("load");
        monitor(AppEvent::verbose(scope.clone(), LoadEvent::Started));

        let loaded = self.target.load().map_err(|e| vec![e])?;

        monitor(AppEvent::verbose(scope.clone(), LoadEvent::Validating));
        // ここで将来的に Validatable トレイトと連携させることも可能

        monitor(AppEvent::verbose(scope, LoadEvent::Completed));

        Ok(loaded)
    }
}
