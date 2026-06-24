use crate::artifact::Artifact;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ArtifactResult<A>
where
    A: Artifact,
{
    pub artifact: A,
    pub success: bool,
    pub error_message: Option<String>,
}

impl<A> ArtifactResult<A>
where
    A: Artifact,
{
    pub fn success(artifact: A) -> Self {
        Self {
            artifact,
            success: true,
            error_message: None,
        }
    }

    pub fn failure(artifact: A, error_message: impl Into<String>) -> Self {
        Self {
            artifact,
            success: false,
            error_message: Some(error_message.into()),
        }
    }

    pub fn artifact(&self) -> &A {
        &self.artifact
    }

    pub fn artifact_mut(&mut self) -> &mut A {
        &mut self.artifact
    }

    pub fn is_success(&self) -> bool {
        self.success
    }

    pub fn error(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    pub fn logical_artifact_root_path(&self) -> PathBuf {
        self.artifact.logical_artifact_root_path()
    }

    pub fn absolute_artifact_root_path(&self) -> PathBuf {
        self.artifact.absolute_artifact_root_path()
    }

    pub fn files(&self) -> Result<Vec<PathBuf>, A::Error> {
        self.artifact.files()
    }
}

#[cfg(test)]
mod tests {
    use super::ArtifactResult;
    use crate::artifact::Artifact;
    use std::path::PathBuf;

    #[derive(Clone, Debug)]
    struct DummyArtifact;

    impl Artifact for DummyArtifact {
        type Error = std::io::Error;

        fn logical_artifact_root_path(&self) -> PathBuf {
            PathBuf::from("dummy")
        }

        fn absolute_artifact_root_path(&self) -> PathBuf {
            PathBuf::from("/tmp/dummy")
        }

        fn files(&self) -> Result<Vec<PathBuf>, Self::Error> {
            Ok(vec![PathBuf::from("/tmp/dummy/main.pdf")])
        }
    }

    #[test]
    fn test_result_delegates_to_inner_artifact() {
        let result = ArtifactResult::success(DummyArtifact);

        assert!(result.is_success());
        assert_eq!(result.error(), None);
        assert_eq!(result.logical_artifact_root_path(), PathBuf::from("dummy"));
        assert_eq!(
            result.absolute_artifact_root_path(),
            PathBuf::from("/tmp/dummy")
        );
        assert_eq!(
            result.files().unwrap(),
            vec![PathBuf::from("/tmp/dummy/main.pdf")]
        );
    }
}
