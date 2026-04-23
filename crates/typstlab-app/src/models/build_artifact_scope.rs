use std::path::PathBuf;
use typstlab_proto::Entity;

/// 成果物が生成される「領土」を管理するモデル
pub struct BuildArtifactScope {
    pub project_root: PathBuf,
    pub relative_path: String, // "dist" など
}

impl BuildArtifactScope {
    pub fn new(project_root: PathBuf, relative_path: String) -> Self {
        Self { project_root, relative_path }
    }

    /// 特定の論文用の成果物ディレクトリ（領土内の区画）を計算
    pub fn paper_dist_path(&self, paper_id: &str) -> PathBuf {
        self.path().join(paper_id)
    }
}

impl Entity for BuildArtifactScope {
    fn path(&self) -> PathBuf {
        self.project_root.join(&self.relative_path)
    }
}
