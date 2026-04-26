use std::path::PathBuf;

/// Typstlab の世界におけるあらゆる「もの（実体・概念）」の基底プロトコル
pub trait Model {}

/// 実体（Model）が備えるべき物理法則（ローカルに実在するもの）
pub trait Entity: Model {
    fn path(&self) -> PathBuf;
    fn exists(&self) -> bool {
        self.path().exists()
    }
}

/// 実体の所在を表現する
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location<T> {
    /// ローカルに実在する（物理パスを持つ）
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

/// 外部リソースを解決するためのプロトコル
pub trait Remote<T, Error>
where
    T: Model,
    Error: std::error::Error + 'static,
{
    /// 識別子から、解決可能なリソースを特定する
    fn resolve_remote(&self, id: &str) -> Result<Option<T>, Error>;
}
