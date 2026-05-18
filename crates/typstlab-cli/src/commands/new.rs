use anyhow::{Result, anyhow};
use colored::Colorize;
use std::path::{Component, Path, PathBuf};
use typstlab_app::{
    CreateAction, CreateError, CreateEvent, Project, ProjectConfig, ProjectCreationArgs,
};
use typstlab_proto::{Action, AppEvent, CliSpeaker, Entity, Loaded, PROJECT_SETTING_FILE};

#[derive(Debug, Clone)]
pub enum NewWarning {
    ExistingProjectSettings { path: PathBuf },
}

fn detect_new_warning(target_path: &Path) -> Result<Option<NewWarning>> {
    let config_path = target_path.join(PROJECT_SETTING_FILE);
    match config_path.try_exists()? {
        true => Ok(Some(NewWarning::ExistingProjectSettings {
            path: config_path,
        })),
        false => Ok(None),
    }
}

fn resolve_target_path(current_dir: &Path, path: Option<&str>) -> PathBuf {
    let target_path = match path {
        None => current_dir.to_path_buf(),
        Some(p) => {
            let p = Path::new(p);
            let has_absolute_or_rooted_component = matches!(
                p.components().next(),
                Some(Component::RootDir | Component::Prefix(_))
            );

            if has_absolute_or_rooted_component {
                p.to_path_buf()
            } else {
                current_dir.join(p)
            }
        }
    };

    normalize_path_lexically(&target_path)
}

fn normalize_path_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }

    normalized
}

fn project_name_from_target_path(target_path: &Path) -> Result<String> {
    target_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| {
            anyhow!(
                "project target path must have a directory name: {}",
                target_path.display()
            )
        })
}

/// new コマンドのエントリポイント
pub fn run(path: Option<String>, verbose: bool) -> Result<()> {
    let current_dir = std::env::current_dir()?;

    // 1. 作成場所の決定
    let target_path = resolve_target_path(&current_dir, path.as_deref());

    // 2. プロジェクト名の決定
    let project_name = project_name_from_target_path(&target_path)?;

    let warning = detect_new_warning(&target_path)?;

    // 3. 実体とアクションの生成
    let project = Project::new(target_path);
    let args = ProjectCreationArgs { name: project_name };
    let action = CreateAction {
        target: project,
        args,
    };
    let presenter = NewPresenter;

    if let Some(warning) = warning {
        presenter.render_warning(warning);
    }

    // 4. 実行
    match action.run(
        &mut |event| {
            if event.visible_in_cli(verbose) {
                presenter.render_event(event);
            }
        },
        &mut |_| {},
    ) {
        Ok(loaded_project) => {
            // パス移動や . を解決した「綺麗な絶対パス」を持つ実体を再生成して結果を表示
            let clean_root = std::fs::canonicalize(loaded_project.path())
                .unwrap_or_else(|_| loaded_project.path());

            let clean_loaded_project = Loaded {
                actual: Project { root: clean_root },
                config: loaded_project.config,
            };

            presenter.render_result(&clean_loaded_project);
            Ok(())
        }
        Err(errors) => {
            for err in errors {
                presenter.render_error(&err);
            }
            Err(anyhow!("Failed to create new project"))
        }
    }
}

struct NewPresenter;

impl CliSpeaker for NewPresenter {
    type Event = CreateEvent;
    type Warning = NewWarning;
    type Error = CreateError;
    type Output = Loaded<Project, ProjectConfig>;

    fn render_event(&self, event: AppEvent<CreateEvent>) {
        match event.payload {
            CreateEvent::Initializing => {
                println!("{} Initializing project structure...", "🐣".cyan());
            }
            CreateEvent::Persisting => {
                println!("{} Writing configuration and directories...", "📝".cyan());
            }
            CreateEvent::Completed => {
                println!("{} Done!", "✨".green());
            }
        }
    }

    fn render_warning(&self, warning: NewWarning) {
        match warning {
            NewWarning::ExistingProjectSettings { path } => {
                eprintln!(
                    "{} Existing project setting file will be reused or overwritten: {}",
                    "⚠".yellow().bold(),
                    path.display()
                );
            }
        }
    }

    fn render_error(&self, error: &CreateError) {
        eprintln!("{} {}", "❌".red(), error);
    }

    fn render_result(&self, loaded_project: &Loaded<Project, ProjectConfig>) {
        println!("\n{} Project created successfully!", "🎉".green().bold());
        // 解決済みの綺麗なパスを表示
        println!(
            "  Location: {}",
            loaded_project.path().display().to_string().cyan()
        );
        println!("\nNext steps:");
        println!("  1. cd {}", loaded_project.path().display());
        println!("  2. typstlab build");
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NewWarning, detect_new_warning, project_name_from_target_path, resolve_target_path,
    };
    use std::path::Path;
    use tempfile::TempDir;
    use typstlab_proto::PROJECT_SETTING_FILE;

    #[test]
    fn test_detect_new_warning_returns_none_when_project_setting_missing() {
        let temp = TempDir::new().unwrap();

        let warning = detect_new_warning(temp.path()).unwrap();

        assert!(warning.is_none());
    }

    #[test]
    fn test_detect_new_warning_returns_existing_project_settings_warning() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(PROJECT_SETTING_FILE);
        std::fs::write(&config_path, "").unwrap();

        let warning = detect_new_warning(temp.path()).unwrap();

        match warning {
            Some(NewWarning::ExistingProjectSettings { path }) => {
                assert_eq!(path, config_path);
            }
            None => panic!("expected warning"),
        }
    }

    #[test]
    fn test_project_name_comes_from_target_directory_when_name_and_path_are_given() {
        let temp = TempDir::new().unwrap();
        let target_path = resolve_target_path(temp.path(), Some("actual-dir"));

        let project_name = project_name_from_target_path(&target_path).unwrap();

        assert_eq!(target_path, temp.path().join("actual-dir"));
        assert_eq!(project_name, "actual-dir");
    }

    #[test]
    fn test_project_name_comes_from_current_directory_when_no_target_is_given() {
        let current_dir = Path::new("/workspace/current-project");
        let target_path = resolve_target_path(current_dir, None);

        let project_name = project_name_from_target_path(&target_path).unwrap();

        assert_eq!(target_path, current_dir);
        assert_eq!(project_name, "current-project");
    }

    #[test]
    fn test_project_name_uses_normalized_target_directory() {
        let current_dir = Path::new("/workspace/current-project");
        let target_path = resolve_target_path(current_dir, Some("../other-project/./nested/.."));

        let project_name = project_name_from_target_path(&target_path).unwrap();

        assert_eq!(target_path, Path::new("/workspace/other-project"));
        assert_eq!(project_name, "other-project");
    }
}
