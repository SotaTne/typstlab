use std::fmt::Debug;
use typstlab_proto::{Action, Loadable, Loaded};

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

impl<T: Loadable> Action<Loaded<T, T::Config>, LoadEvent, T::Error> for LoadAction<T> {
    fn run(
        self,
        monitor: &mut dyn FnMut(LoadEvent),
    ) -> Result<Loaded<T, T::Config>, Vec<T::Error>> {
        monitor(LoadEvent::Started);

        let loaded = self.target.load().map_err(|e| vec![e])?;

        monitor(LoadEvent::Validating);
        // ここで将来的に Validatable トレイトと連携させることも可能

        monitor(LoadEvent::Completed);

        Ok(loaded)
    }
}
