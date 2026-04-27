use std::path::{Path, PathBuf};

use tempfile::TempDir;
use thiserror::Error;

use crate::Persistence;

const PROJECT_CACHE_DIR: &str = ".typstlab";
const PROJECT_TMP_DIR: &str = ".tmp";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectDocs {
    Typst,
}

impl ProjectDocs {
    fn path_name(self) -> &'static str {
        match self {
            Self::Typst => "typst_docs",
        }
    }
}

#[derive(Debug, Error)]
pub enum ProjectDocsSyncError {
    #[error("source docs path does not exist: {0}")]
    SourceMissing(PathBuf),
    #[error("failed to create project docs staging area: {0}")]
    Staging(#[source] std::io::Error),
    #[error("failed to copy project docs: {0}")]
    Copy(#[source] std::io::Error),
    #[error("failed to replace project docs: {0}")]
    Replace(#[source] ProjectDocsCommitError),
}

#[derive(Debug, Error)]
pub enum ProjectDocsCommitError {
    #[error("failed to prepare project docs target parent: {0}")]
    PrepareParent(#[source] std::io::Error),
    #[error("failed to create backup directory for existing project docs: {0}")]
    CreateBackup(#[source] std::io::Error),
    #[error("failed to move existing project docs to backup: {0}")]
    MoveTargetToBackup(#[source] std::io::Error),
    #[error("failed to commit staged project docs: {0}")]
    MoveStagingToTarget(#[source] std::io::Error),
    #[error("failed to commit staged project docs: {commit}; rollback also failed: {rollback}")]
    RollbackFailed {
        #[source]
        commit: std::io::Error,
        rollback: std::io::Error,
    },
    #[error("failed to remove old project docs backup after commit: {0}")]
    RemoveBackup(#[source] std::io::Error),
}

pub fn sync_project_docs(
    project_root: impl AsRef<Path>,
    docs: ProjectDocs,
    source_path: impl AsRef<Path>,
) -> Result<PathBuf, ProjectDocsSyncError> {
    let project_root = project_root.as_ref();
    let source_path = source_path.as_ref();
    if !source_path.is_dir() {
        return Err(ProjectDocsSyncError::SourceMissing(
            source_path.to_path_buf(),
        ));
    }

    let target_path = project_docs_path(project_root, docs);
    let staging = create_staging_area(project_root, docs)?;
    copy_docs_into_staging(source_path, staging.path()).map_err(ProjectDocsSyncError::Copy)?;
    commit_staging_with_fs(staging.path(), &target_path, &StdProjectDocsCommitFs)
        .map_err(ProjectDocsSyncError::Replace)?;

    Ok(target_path)
}

pub fn project_docs_path(project_root: &Path, docs: ProjectDocs) -> PathBuf {
    project_root.join(PROJECT_CACHE_DIR).join(docs.path_name())
}

fn create_staging_area(
    project_root: &Path,
    docs: ProjectDocs,
) -> Result<TempDir, ProjectDocsSyncError> {
    Persistence::create_temp_dir(
        project_root.join(PROJECT_CACHE_DIR).join(PROJECT_TMP_DIR),
        &format!("staging-{}-", docs.path_name()),
    )
    .map_err(|error| ProjectDocsSyncError::Staging(std::io::Error::other(error)))
}

trait ProjectDocsCommitFs {
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn create_backup_dir(&self, target_path: &Path) -> std::io::Result<TempDir>;
    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()>;
    fn remove_dir_all(&self, path: &Path) -> std::io::Result<()>;
}

struct StdProjectDocsCommitFs;

impl ProjectDocsCommitFs for StdProjectDocsCommitFs {
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn create_backup_dir(&self, target_path: &Path) -> std::io::Result<TempDir> {
        create_backup_dir(target_path)
    }

    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        std::fs::rename(from, to)
    }

    fn remove_dir_all(&self, path: &Path) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }
}

fn commit_staging_with_fs(
    staging_path: &Path,
    target_path: &Path,
    fs: &impl ProjectDocsCommitFs,
) -> Result<(), ProjectDocsCommitError> {
    if let Some(parent) = target_path.parent() {
        fs.create_dir_all(parent)
            .map_err(ProjectDocsCommitError::PrepareParent)?;
    }

    if fs.exists(target_path) {
        let backup = fs
            .create_backup_dir(target_path)
            .map_err(ProjectDocsCommitError::CreateBackup)?;
        let backup_path = backup.path().join("old");
        fs.rename(target_path, &backup_path)
            .map_err(ProjectDocsCommitError::MoveTargetToBackup)?;
        match fs.rename(staging_path, target_path) {
            Ok(()) => {
                fs.remove_dir_all(backup.path())
                    .map_err(ProjectDocsCommitError::RemoveBackup)?;
                Ok(())
            }
            Err(error) => match fs.rename(&backup_path, target_path) {
                Ok(()) => Err(ProjectDocsCommitError::MoveStagingToTarget(error)),
                Err(rollback) => Err(ProjectDocsCommitError::RollbackFailed {
                    commit: error,
                    rollback,
                }),
            },
        }
    } else {
        fs.rename(staging_path, target_path)
            .map_err(ProjectDocsCommitError::MoveStagingToTarget)
    }
}

fn create_backup_dir(target_path: &Path) -> std::io::Result<TempDir> {
    let parent = target_path
        .parent()
        .ok_or_else(|| std::io::Error::other("project docs target has no parent"))?;
    let tmp_root = parent.join(PROJECT_TMP_DIR);
    std::fs::create_dir_all(&tmp_root)?;
    tempfile::Builder::new()
        .prefix(".old-project-docs-")
        .tempdir_in(tmp_root)
}

fn copy_docs_into_staging(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_docs_into_staging(&source, &target)?;
        } else {
            std::fs::copy(&source, &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_sync_project_docs_copies_typst_docs_into_project_cache() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path().join("project");
        let source = temp.path().join("source");
        std::fs::create_dir_all(source.join("tutorial")).unwrap();
        std::fs::write(source.join("index.md"), "# Overview").unwrap();
        std::fs::write(source.join("tutorial").join("writing.md"), "# Writing").unwrap();

        let docs_path = sync_project_docs(&project_root, ProjectDocs::Typst, &source).unwrap();

        assert_eq!(docs_path, project_root.join(".typstlab").join("typst_docs"));
        assert!(docs_path.join("index.md").exists());
        assert!(docs_path.join("tutorial").join("writing.md").exists());
    }

    #[test]
    fn test_sync_project_docs_replaces_existing_typst_docs() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path().join("project");
        let old_docs = project_root.join(".typstlab").join("typst_docs");
        std::fs::create_dir_all(&old_docs).unwrap();
        std::fs::write(old_docs.join("old.md"), "# Old").unwrap();
        let source = temp.path().join("source");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("index.md"), "# New").unwrap();

        let docs_path = sync_project_docs(&project_root, ProjectDocs::Typst, &source).unwrap();

        assert!(docs_path.join("index.md").exists());
        assert!(!docs_path.join("old.md").exists());
    }

    #[test]
    fn test_sync_project_docs_rejects_missing_source_path() {
        let temp = TempDir::new().unwrap();

        let error = sync_project_docs(
            temp.path().join("project"),
            ProjectDocs::Typst,
            temp.path().join("missing"),
        )
        .unwrap_err();

        assert!(matches!(error, ProjectDocsSyncError::SourceMissing(_)));
    }

    #[test]
    fn test_uncommitted_prepared_sync_cleans_up_staging_on_drop() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path().join("project");
        let source = temp.path().join("source");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("index.md"), "# Overview").unwrap();

        let staging_path = {
            let staging = create_staging_area(&project_root, ProjectDocs::Typst).unwrap();
            copy_docs_into_staging(&source, staging.path()).unwrap();
            let staging_path = staging.path().to_path_buf();
            assert!(staging_path.exists());
            staging_path
        };

        assert!(!staging_path.exists());
    }

    #[test]
    fn test_sync_project_docs_reports_staging_creation_failure() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path().join("project");
        std::fs::create_dir_all(&project_root).unwrap();
        std::fs::write(project_root.join(".typstlab"), "not a directory").unwrap();
        let source = temp.path().join("source");
        std::fs::create_dir_all(&source).unwrap();

        let error = sync_project_docs(&project_root, ProjectDocs::Typst, &source).unwrap_err();

        assert!(matches!(error, ProjectDocsSyncError::Staging(_)));
    }

    #[test]
    fn test_copy_docs_into_staging_reports_copy_failure() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("index.md"), "# Overview").unwrap();
        let target_file = temp.path().join("target-file");
        std::fs::write(&target_file, "not a directory").unwrap();

        let error = copy_docs_into_staging(&source, &target_file).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn test_commit_staging_reports_prepare_parent_failure() {
        let fs = FakeCommitFs::new().with_prepare_parent_error();
        let error =
            commit_staging_with_fs(Path::new("staging"), Path::new("target"), &fs).unwrap_err();

        assert!(matches!(error, ProjectDocsCommitError::PrepareParent(_)));
    }

    #[test]
    fn test_commit_staging_reports_create_backup_failure() {
        let fs = FakeCommitFs::new()
            .with_target_exists()
            .with_create_backup_error();
        let error =
            commit_staging_with_fs(Path::new("staging"), Path::new("target"), &fs).unwrap_err();

        assert!(matches!(error, ProjectDocsCommitError::CreateBackup(_)));
    }

    #[test]
    fn test_commit_staging_reports_move_target_to_backup_failure() {
        let fs = FakeCommitFs::new()
            .with_target_exists()
            .with_rename_error(0);
        let error =
            commit_staging_with_fs(Path::new("staging"), Path::new("target"), &fs).unwrap_err();

        assert!(matches!(
            error,
            ProjectDocsCommitError::MoveTargetToBackup(_)
        ));
    }

    #[test]
    fn test_commit_staging_reports_move_staging_to_target_failure_after_rollback() {
        let fs = FakeCommitFs::new()
            .with_target_exists()
            .with_rename_error(1);
        let error =
            commit_staging_with_fs(Path::new("staging"), Path::new("target"), &fs).unwrap_err();

        assert!(matches!(
            error,
            ProjectDocsCommitError::MoveStagingToTarget(_)
        ));
    }

    #[test]
    fn test_commit_staging_reports_rollback_failure() {
        let fs = FakeCommitFs::new()
            .with_target_exists()
            .with_rename_error(1)
            .with_rename_error(2);
        let error =
            commit_staging_with_fs(Path::new("staging"), Path::new("target"), &fs).unwrap_err();

        assert!(matches!(
            error,
            ProjectDocsCommitError::RollbackFailed { .. }
        ));
    }

    #[test]
    fn test_commit_staging_reports_remove_backup_failure() {
        let fs = FakeCommitFs::new()
            .with_target_exists()
            .with_remove_backup_error();
        let error =
            commit_staging_with_fs(Path::new("staging"), Path::new("target"), &fs).unwrap_err();

        assert!(matches!(error, ProjectDocsCommitError::RemoveBackup(_)));
    }

    #[derive(Default)]
    struct FakeCommitFs {
        target_exists: bool,
        prepare_parent_error: bool,
        create_backup_error: bool,
        remove_backup_error: bool,
        rename_errors: Vec<usize>,
        rename_calls: RefCell<usize>,
        backup_dir: RefCell<Option<TempDir>>,
    }

    impl FakeCommitFs {
        fn new() -> Self {
            Self::default()
        }

        fn with_target_exists(mut self) -> Self {
            self.target_exists = true;
            self
        }

        fn with_prepare_parent_error(mut self) -> Self {
            self.prepare_parent_error = true;
            self
        }

        fn with_create_backup_error(mut self) -> Self {
            self.create_backup_error = true;
            self
        }

        fn with_remove_backup_error(mut self) -> Self {
            self.remove_backup_error = true;
            self
        }

        fn with_rename_error(mut self, call_index: usize) -> Self {
            self.rename_errors.push(call_index);
            self
        }
    }

    impl ProjectDocsCommitFs for FakeCommitFs {
        fn create_dir_all(&self, _path: &Path) -> std::io::Result<()> {
            if self.prepare_parent_error {
                Err(std::io::Error::other("prepare parent failed"))
            } else {
                Ok(())
            }
        }

        fn exists(&self, _path: &Path) -> bool {
            self.target_exists
        }

        fn create_backup_dir(&self, _target_path: &Path) -> std::io::Result<TempDir> {
            if self.create_backup_error {
                return Err(std::io::Error::other("backup failed"));
            }
            let dir = TempDir::new()?;
            let path = dir.path().to_path_buf();
            *self.backup_dir.borrow_mut() = Some(dir);
            TempDir::new_in(path.parent().unwrap())
        }

        fn rename(&self, _from: &Path, _to: &Path) -> std::io::Result<()> {
            let call_index = *self.rename_calls.borrow();
            *self.rename_calls.borrow_mut() += 1;
            if self.rename_errors.contains(&call_index) {
                Err(std::io::Error::other("rename failed"))
            } else {
                Ok(())
            }
        }

        fn remove_dir_all(&self, _path: &Path) -> std::io::Result<()> {
            if self.remove_backup_error {
                Err(std::io::Error::other("remove backup failed"))
            } else {
                Ok(())
            }
        }
    }
}
