use std::path::PathBuf;
use typstlab_proto::Entity;

/// プロジェクト全体の成果物領土 (例: dist/)
pub struct BuildArtifactScope {
    pub project_root: PathBuf,
    pub relative_path: String,
}

impl BuildArtifactScope {
    pub fn new(project_root: PathBuf, relative_path: String) -> Self {
        Self { project_root, relative_path }
    }

    /// 特定の論文用の領土（区画）を取得
    pub fn paper_scope(&self, paper_id: &str) -> PaperArtifactScope {
        PaperArtifactScope {
            root: self.path().join(paper_id),
            paper_id: paper_id.to_string(),
        }
    }
}

impl Entity for BuildArtifactScope {
    fn path(&self) -> PathBuf {
        self.project_root.join(&self.relative_path)
    }
}

/// 論文単位の成果物領土 (例: dist/p01/)
pub struct PaperArtifactScope {
    pub root: PathBuf,
    pub paper_id: String,
}

impl PaperArtifactScope {
    /// 特定の形式（pdf, png等）の最終的な実体（Artifact）を取得
    pub fn format_artifact(&self, format: &str) -> crate::models::build_artifact::BuildArtifact {
        let (root_name, absolute_path) = if format == "pdf" {
            // pdf なら領土ルート直下 (例: p01/)
            (PathBuf::from(&self.paper_id), self.root.clone())
        } else {
            // それ以外ならサブディレクトリ (例: p01/png/)
            (PathBuf::from(&self.paper_id).join(format), self.root.join(format))
        };

        crate::models::build_artifact::BuildArtifact {
            root_name,
            absolute_path,
            success: false,
            error_message: None,
        }
    }
}

impl Entity for PaperArtifactScope {
    fn path(&self) -> PathBuf {
        self.root.clone()
    }
}
