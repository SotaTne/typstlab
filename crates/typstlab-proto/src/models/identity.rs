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
