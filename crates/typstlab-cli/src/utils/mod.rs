use crate::CliError;
use std::path::{Path, PathBuf};
use typstlab_app::{AppContext, BootstrapAction, BootstrapError};
use typstlab_proto::{Action, PROJECT_SETTING_FILE};

pub fn find_project_root(start: &Path) -> Result<PathBuf, CliError> {
    let mut current = start.to_path_buf();

    loop {
        let config_path = current.join(PROJECT_SETTING_FILE);
        match config_path.try_exists() {
            Ok(true) => return Ok(current),
            Ok(false) => {}
            Err(error) => {
                return Err(CliError::System(format!(
                    "Could not inspect project setting file '{}': {}",
                    config_path.display(),
                    error
                )));
            }
        }

        let Some(parent) = current.parent() else {
            return Err(CliError::ProjectRootNotFound {
                start: start.to_path_buf(),
                config_file: PROJECT_SETTING_FILE,
            });
        };
        current = parent.to_path_buf();
    }
}

pub fn bootstrap_context(
    monitor: &mut dyn FnMut(typstlab_app::BootstrapEvent),
) -> Result<AppContext, CliError> {
    let current_dir = std::env::current_dir()
        .map_err(|error| CliError::System(format!("Could not identify current directory: {}", error)))?;
    let project_root = find_project_root(&current_dir)?;

    let cache_root = dirs::cache_dir()
        .ok_or_else(|| CliError::System("Could not find cache directory".to_string()))?
        .join("typstlab");

    let bootstrap = BootstrapAction {
        project_root,
        cache_root,
    };

    bootstrap
        .run(monitor, &mut |_| {})
        .map_err(collapse_bootstrap_errors)
}

fn collapse_bootstrap_errors(errors: Vec<BootstrapError>) -> CliError {
    let mut iter = errors.into_iter();
    match iter.next() {
        Some(error) => CliError::Bootstrap(error),
        None => CliError::System("Bootstrap failed without an error".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::find_project_root;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use typstlab_proto::PROJECT_SETTING_FILE;

    #[test]
    fn test_find_project_root_walks_up_to_project_setting_file() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path().join("workspace").join("demo");
        let nested_dir = project_root.join("papers").join("p01").join("src");
        std::fs::create_dir_all(&nested_dir).unwrap();
        std::fs::write(project_root.join(PROJECT_SETTING_FILE), "{}",).unwrap();

        let detected_root = find_project_root(&nested_dir).unwrap();

        assert_eq!(detected_root, project_root);
    }

    #[test]
    fn test_find_project_root_errors_when_project_setting_file_missing() {
        let temp = TempDir::new().unwrap();
        let start = temp.path().join("workspace").join("demo").join("papers");
        std::fs::create_dir_all(&start).unwrap();

        let error = find_project_root(&start).unwrap_err();

        match error {
            crate::CliError::ProjectRootNotFound {
                start: actual,
                config_file,
            } => {
                assert_eq!(actual, PathBuf::from(&start));
                assert_eq!(config_file, PROJECT_SETTING_FILE);
            }
            other => panic!("unexpected error: {}", other),
        }
    }
}
