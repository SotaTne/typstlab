use std::path::PathBuf;
use typstlab_proto::{Entity, Artifact};

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
    fn root(&self) -> PathBuf {
        self.root_name.clone()
    }

    fn is_success(&self) -> bool {
        self.success
    }

    fn error(&self) -> Option<String> {
        self.error_message.clone()
    }

    fn files(&self) -> Result<Vec<PathBuf>, String> {
        let mut files = Vec::new();
        if self.absolute_path.exists() {
            for entry in std::fs::read_dir(&self.absolute_path).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.path().is_file() {
                    files.push(entry.path());
                }
            }
        }
        Ok(files)
    }
}
