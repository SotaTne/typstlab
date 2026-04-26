use crate::model::{Entity, Model};
use std::path::PathBuf;

pub struct Loaded<Actual, Config>
where
    Actual: Model,
{
    pub actual: Actual,
    pub config: Config,
}

impl<Actual, Config> Model for Loaded<Actual, Config> where Actual: Model {}

impl<Actual, Config> Entity for Loaded<Actual, Config>
where
    Actual: Entity,
{
    fn path(&self) -> PathBuf {
        self.actual.path()
    }
}

/// モデルの集合（ディレクトリ等）を扱うプロトコル
pub trait Collection<T, Error>: Entity
where
    T: Model,
    Error: std::error::Error + 'static,
{
    fn list(&self) -> Result<Vec<T>, Error>;
    fn resolve(&self, input: &str) -> Result<Option<T>, Error>;
}

/// 実体の永続化・管理を担う保管庫のプロトコル
pub trait Store<T, Error>: Collection<T, Error>
where
    T: Model,
    Error: std::error::Error + 'static,
{
    /// ステージングエリアを表現する型。
    /// パスとして参照可能であり、ドロップ（スコープを抜ける）時に自動的に物理ディレクトリを削除することが期待される。
    type Staging: AsRef<std::path::Path>;

    /// 実体を準備するための、他と隔離された安全な一時作業場所（Staging Area）を作成してその所有権を返す。
    fn create_staging_area(&self, id: &str) -> Result<Self::Staging, Error>;

    /// ステージングエリアで完成した実体を、正式な領土へアトミックに移動（確定）し、実体を返す。
    fn commit_staged(&self, id: &str, staging: Self::Staging) -> Result<T, Error>;
}

pub trait Artifact: Entity {
    type Error: std::error::Error;

    fn root(&self) -> PathBuf;
    fn is_success(&self) -> bool;
    fn error(&self) -> Option<String>;
    fn files(&self) -> Result<Vec<PathBuf>, Self::Error>;
}
