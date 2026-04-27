use std::path::Path;

/// 外部リソースの取得形式。
///
/// この値は入力リソースの形式だけを表す。戻り値の形は表さない。
/// どの形式でも、`Installer` は所有された一時領域へ内容を実体化して返す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceFormat {
    /// `.tar.xz` アーカイブ。インストーラーは一時インストール領域へ展開する。
    TarXz { strip_components: usize },
    /// `.zip` アーカイブ。インストーラーは一時インストール領域へ展開する。
    Zip { strip_components: usize },
    /// 生バイト列。インストーラーは一時インストール領域内のファイルへ書き込む。
    Raw,
}

/// 外部リソースを取得し、所有された一時インストール領域へ配置するプロトコル。
pub trait Installer: Send + Sync {
    /// インストーラー固有のエラー型。
    type Error: std::error::Error + Send + Sync + 'static;
    /// インストール結果を所有する一時領域。
    ///
    /// この値を drop すると、まだ commit されていないインストール出力が cleanup される。
    /// 通常の実装では `tempfile::TempDir` を使う。
    type Installation: AsRef<Path>;

    /// `url` からリソースを取得し、所有された一時インストール領域へ実体化して返す。
    ///
    /// `on_progress` は `(current, total)` の生バイト進捗を受け取る。
    /// この callback は意図的にバイト指向であり、download / transform / install /
    /// commit のような UI 上の phase は知らない。
    fn install<F>(
        &self,
        url: &str,
        format: SourceFormat,
        on_progress: F,
    ) -> Result<Self::Installation, Self::Error>
    where
        F: FnMut(u64, u64) + Send + 'static;
}
