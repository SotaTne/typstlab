use crate::models::identity::Entity;
use std::path::PathBuf;

/// 実行結果としてのモデル
pub trait Artifact: Entity {
    type Error: std::error::Error;

    fn root(&self) -> PathBuf;
    fn is_success(&self) -> bool;
    fn error(&self) -> Option<String>;
    fn files(&self) -> Result<Vec<PathBuf>, Self::Error>;
}
