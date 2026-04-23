use typstlab_proto::{Action, Loadable};
use std::fmt::Debug;

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

impl<T: Loadable> Action<T, LoadEvent, T::Error> for LoadAction<T> {
    fn run(mut self, monitor: &mut dyn FnMut(LoadEvent)) -> Result<T, Vec<T::Error>> {
        monitor(LoadEvent::Started);
        
        // Loadable プロトコルの reload (load_from_disk + apply_config) を実行
        self.target.reload().map_err(|e| vec![e])?;

        monitor(LoadEvent::Validating);
        // ここで将来的に Validatable トレイトと連携させることも可能
        
        monitor(LoadEvent::Completed);

        Ok(self.target)
    }
}
