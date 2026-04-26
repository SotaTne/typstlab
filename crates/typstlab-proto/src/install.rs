use std::path::{Path, PathBuf};

/// インストール対象のソース形式
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceFormat {
    /// .tar.xz 形式 (解凍してディレクトリを構成)
    TarXz { strip_components: usize },
    /// .zip 形式 (解凍してディレクトリを構成)
    Zip { strip_components: usize },
    /// 生データ (解凍せず、Reader として提供)
    Raw,
}

/// ダウンロード（および展開）の結果
pub enum Downloaded {
    /// すでに指定のディレクトリに展開・配置された
    Archive(PathBuf),
    /// 生データとしてストリームで提供される状態（Readerを保持）
    Raw(Box<dyn std::io::Read + Send>),
}

/// 外部リソースを物理的に取得して配置するプロトコル
pub trait Installer: Send + Sync {
    /// 実装側が定義する、そのインストーラー特有のエラー型
    type Error: std::error::Error + Send + Sync + 'static;

    /// 指定された URL からリソースを取得し、必要に応じて展開する。
    /// progress コールバック: (現在のバイト数, 全体のバイト数)
    fn install<F>(
        &self,
        url: &str,
        format: SourceFormat,
        dest: &Path,
        on_progress: F,
    ) -> Result<Downloaded, Self::Error>
    where
        F: FnMut(u64, u64) + Send + 'static;
}
