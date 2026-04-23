use crate::models::Paper;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;
use typstlab_proto::{Collection, Entity};

#[derive(Error, Debug)]
pub enum CollectionError {
    #[error("IO error while scanning collection: {0}")]
    Io(#[from] std::io::Error),
    #[error("Base directory not found: {0}")]
    NotFound(PathBuf),
    #[error("Target is outside of the collection scope: {0}")]
    OutsideScope(PathBuf),
}

pub struct PaperScope {
    pub project_root: PathBuf,
    pub relative_path: PathBuf,
}

impl PaperScope {
    pub fn new(project_root: PathBuf, relative_path: PathBuf) -> Self {
        Self {
            project_root,
            relative_path,
        }
    }
}

impl Entity for PaperScope {
    fn path(&self) -> PathBuf {
        self.project_root.join(&self.relative_path)
    }
}

impl Collection<Paper, CollectionError> for PaperScope {
    fn list(&self) -> Result<Vec<Paper>, CollectionError> {
        let root = self.path();
        if !root.exists() {
            return Err(CollectionError::NotFound(root));
        }

        let mut papers = Vec::new();
        for entry in std::fs::read_dir(&root)? {
            let entry = entry?;
            if !entry.path().is_dir() {
                continue;
            }

            if let Some(id) = entry.file_name().to_str() {
                papers.push(Paper::new(id.to_string(), root.clone()));
            }
        }
        Ok(papers)
    }

    fn resolve(&self, input: &str) -> Option<Paper> {
        let input_path = Path::new(input);
        let scope_root = self.path();

        // 1. ID として直接存在するかチェック
        let potential_paper = Paper::new(input.to_string(), scope_root.clone());
        if potential_paper.path().exists() {
            return Some(potential_paper);
        }

        // 2. パスとして解決を試みる
        let has_absolute_or_rooted_component = matches!(
            input_path.components().next(),
            Some(Component::RootDir | Component::Prefix(_))
        );

        let abs_input = if has_absolute_or_rooted_component {
            input_path.to_path_buf()
        } else {
            self.project_root.join(input_path)
        };

        let full_input = std::fs::canonicalize(&abs_input).ok()?;
        let full_root = std::fs::canonicalize(&scope_root).ok()?;

        if full_input.starts_with(&full_root) {
            let relative = full_input.strip_prefix(&full_root).ok()?;
            let id = relative.components().next()?.as_os_str().to_str()?;
            return Some(Paper::new(id.to_string(), scope_root));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::PaperScope;
    use std::path::PathBuf;
    use typstlab_proto::Entity;

    #[test]
    fn test_path_supports_nested_relative_path() {
        let root = PathBuf::from("/project-root");
        let scope = PaperScope::new(root.clone(), PathBuf::from("content").join("papers"));

        assert_eq!(scope.path(), root.join("content").join("papers"));
    }
}
