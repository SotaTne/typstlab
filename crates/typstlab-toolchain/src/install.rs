use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    TarXz { strip_components: usize },
    Zip { strip_components: usize },
    Raw,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallLayout {
    pub source_format: SourceFormat,
    /// `source_format` の strip 適用後の install root から見た実行ファイルの相対パス。
    pub executable_relative_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallSource {
    pub url: String,
    pub layout: InstallLayout,
}
