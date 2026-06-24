use std::path::PathBuf;

/// 実行結果としてのモデル
pub trait Artifact {
    type Error: std::error::Error;

    fn logical_artifact_root_path(&self) -> PathBuf;
    fn absolute_artifact_root_path(&self) -> PathBuf;
    fn files(&self) -> Result<Vec<PathBuf>, Self::Error>;
}
