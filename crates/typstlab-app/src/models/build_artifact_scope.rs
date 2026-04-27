use crate::models::BuildArtifact;
use std::path::PathBuf;
use typstlab_proto::Entity;

pub struct BuildArtifactScope {
    pub project_root: PathBuf,
    pub relative_path: PathBuf,
}

typstlab_proto::impl_entity! {
    BuildArtifactScope {
        fn path(&self) -> PathBuf {
            self.project_root.join(&self.relative_path)
        }
    }
}

impl BuildArtifactScope {
    pub fn new(project_root: PathBuf, relative_path: PathBuf) -> Self {
        Self {
            project_root,
            relative_path,
        }
    }

    pub fn paper_scope(&self, paper_id: &str) -> PaperArtifactScope {
        PaperArtifactScope {
            paper_id: paper_id.to_string(),
            root: self.path().join(paper_id),
        }
    }
}

pub struct PaperArtifactScope {
    pub paper_id: String,
    pub root: PathBuf,
}

typstlab_proto::impl_entity! {
    PaperArtifactScope {
        fn path(&self) -> PathBuf {
            self.root.clone()
        }
    }
}

impl PaperArtifactScope {
    pub fn format_artifact(&self, format: &str) -> BuildArtifact {
        let (root_name, absolute_path) = if format == "pdf" {
            // pdf なら領土ルート直下 (例: p01/)
            (PathBuf::from(&self.paper_id), self.root.clone())
        } else {
            // それ以外ならサブディレクトリ (例: p01/png/)
            (
                PathBuf::from(&self.paper_id).join(format),
                self.root.join(format),
            )
        };

        BuildArtifact {
            root_name,
            absolute_path,
            success: false,
            error_message: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BuildArtifactScope;
    use std::path::PathBuf;
    use typstlab_proto::Entity;

    #[test]
    fn test_path_supports_nested_relative_path() {
        let root = PathBuf::from("/project-root");
        let scope =
            BuildArtifactScope::new(root.clone(), PathBuf::from("target").join("artifacts"));

        assert_eq!(scope.path(), root.join("target").join("artifacts"));
    }
}
