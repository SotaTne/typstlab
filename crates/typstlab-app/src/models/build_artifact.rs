use std::path::PathBuf;
use typstlab_proto::{Artifact, Entity};

/// ビルドの結果として生まれた事実を表す実体（証備）
#[derive(Debug, Clone)]
pub struct BuildArtifact {
    pub root_name: PathBuf,    // 論理的なルート名 ("p01" や "p01/png")
    pub absolute_path: PathBuf, // 実際の絶対パス
    pub success: bool,
    pub error_message: Option<String>,
}

impl Entity for BuildArtifact {
    fn path(&self) -> PathBuf {
        self.absolute_path.clone()
    }
}

impl Artifact for BuildArtifact {
    type Error = std::io::Error;

    fn root(&self) -> PathBuf {
        self.root_name.clone()
    }

    fn is_success(&self) -> bool {
        self.success
    }

    fn error(&self) -> Option<String> {
        self.error_message.clone()
    }

    fn files(&self) -> Result<Vec<PathBuf>, Self::Error> {
        let mut files = Vec::new();
        if self.absolute_path.exists() {
            for entry in std::fs::read_dir(&self.absolute_path)? {
                let entry = entry?;
                if entry.path().is_file() {
                    files.push(entry.path());
                }
            }
        }
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::BuildArtifact;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use typstlab_proto::Artifact;

    #[test]
    fn test_files_lists_children_in_existing_directory() {
        let temp = TempDir::new().unwrap();
        let artifact_dir = temp.path().join("dist").join("p01");
        std::fs::create_dir_all(&artifact_dir).unwrap();
        std::fs::write(artifact_dir.join("main.pdf"), b"pdf").unwrap();
        std::fs::create_dir_all(artifact_dir.join("nested")).unwrap();

        let artifact = BuildArtifact {
            root_name: PathBuf::from("p01"),
            absolute_path: artifact_dir.clone(),
            success: true,
            error_message: None,
        };

        let files = artifact.files().unwrap();

        assert_eq!(files, vec![artifact_dir.join("main.pdf")]);
    }
}
