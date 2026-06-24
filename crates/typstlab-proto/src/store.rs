use std::path::{Path, PathBuf};

use crate::path::HasRoot;

/// Store root 配下で、key が指す保存済み entry の位置を表す。
///
/// relative_path は正式な保存先、staging_prefix は commit 前の一時作業領域名に使う。
pub trait StoreEntryAddress {
    fn relative_path(&self) -> PathBuf;
    fn staging_prefix(&self) -> String;
}

/// 副作用のある永続化境界。
///
/// Store は写像ではなく、staging から commit への原子的な状態遷移を担当する。
pub trait Store: HasRoot {
    /// key によって特定される保存済み実体。
    ///
    /// Item 自身は単体ファイルではなく、それを特定できる root directory と、
    /// その配下にある本体を表す。
    type Item: HasRoot;
    type Key: StoreEntryAddress + ?Sized;
    type Error: std::error::Error + 'static;
    type Staging: AsRef<Path>;

    /// Store root と key の relative path から、正式な保存先 path を返す。
    fn path_for(&self, key: &Self::Key) -> PathBuf {
        self.root().join(key.relative_path())
    }

    /// 保存済みの実体を key から解決する。
    fn resolve(&self, key: &Self::Key) -> Result<Option<Self::Item>, Self::Error>;

    /// commit 前の一時作業領域を作成する。
    fn stage(&self, key: &Self::Key) -> Result<Self::Staging, Self::Error>;

    /// 完成済み staging を正式な保存先へ反映し、保存済み実体を返す。
    fn commit(&self, key: &Self::Key, staging: Self::Staging) -> Result<Self::Item, Self::Error>;
}

/// Store 内の一覧取得。
///
/// filesystem scan などの IO が発生し得るため、HasEntities とは分ける。
pub trait StoreIndex: Store {
    fn list(&self) -> Result<Vec<Self::Item>, Self::Error>;
}
