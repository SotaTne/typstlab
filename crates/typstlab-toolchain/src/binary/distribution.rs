use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryArchiveFormat {
    /// tar.xz archive。strip_components は展開時に先頭から捨てる path component 数。
    TarXz { strip_components: usize },
    /// zip archive。strip_components は展開時に先頭から捨てる path component 数。
    Zip { strip_components: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryLayout {
    /// 配布 archive の形式と、展開時に必要な path 補正。
    pub archive_format: BinaryArchiveFormat,
    /// `archive_format` の strip 適用後の install root から見た実行ファイルの相対パス。
    pub executable_relative_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryDistribution {
    /// version と platform から確定した download URL。
    pub url: String,
    /// download 後に binary をどこへ配置・解決すればよいかを表す layout。
    pub layout: BinaryLayout,
}
