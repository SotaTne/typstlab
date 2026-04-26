use crate::models::identity::{Entity, Model};

/// 集合知としてのモデル
pub trait Collection<T, Error>: Entity
where
    T: Model,
    Error: std::error::Error + 'static,
{
    fn list(&self) -> Result<Vec<T>, Error>;
    fn resolve(&self, input: &str) -> Result<Option<T>, Error>;
}

/// 原子操作を保証する保管庫
pub trait Store<T, Error>: Collection<T, Error>
where
    T: Model,
    Error: std::error::Error + 'static,
{
    /// ステージングエリアを表現する型。
    type Staging: AsRef<std::path::Path>;

    /// 安全な一時作業場所（Staging Area）を作成してその所有権を返す。
    fn create_staging_area(&self, id: &str) -> Result<Self::Staging, Error>;

    /// 作業場所で完成した実体を、正式な領土へアトミックに移動（確定）し、実体を返す。
    fn commit_staged(&self, id: &str, staging: Self::Staging) -> Result<T, Error>;
}
