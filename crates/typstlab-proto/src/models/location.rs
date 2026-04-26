use crate::models::identity::Model;
use std::path::PathBuf;

/// 実体の所在を表現する
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location<T> {
    /// ローカルに実在する
    Local(PathBuf),
    /// 外部に存在する（型 T で定義された識別情報を持つ）
    Remote(T),
}

impl<T> Location<T> {
    /// ローカルであれば PathBuf を返し、リモートであれば None を返す
    pub fn as_local_path(&self) -> Option<&PathBuf> {
        match self {
            Location::Local(path) => Some(path),
            Location::Remote(_) => None,
        }
    }
}

/// 所在を知っていることを示すプロトコル
pub trait Locatable<T>: Model {
    fn location(&self) -> Location<T>;
}

/// 外部からの発見を担うプロトコル
pub trait Remote<T, Error>
where
    T: Model,
    Error: std::error::Error + 'static,
{
    /// 識別子から、解決可能なリソースを特定する
    fn resolve_remote(&self, id: &str) -> Result<Option<T>, Error>;
}
