use anyhow::{Result, anyhow};
use colored::Colorize;
use std::path::{Component, Path};
use typstlab_app::{
    CreateAction, CreateError, CreateEvent, Project, ProjectConfig, ProjectCreationArgs,
};
use typstlab_proto::{Action, CliSpeaker, Entity, Loaded};

/// new コマンドのエントリポイント
pub fn run(name: Option<String>, path: Option<String>) -> Result<()> {
    let current_dir = std::env::current_dir()?;

    // 1. 作成場所の決定
    let target_path = match (&name, &path) {
        // 名前もパスもなし -> カレントディレクトリ
        (None, None) => current_dir.clone(),
        // 名前あり、パスなし -> カレントディレクトリの下に名前で作成
        (Some(n), None) => current_dir.join(n),
        // パス指定あり -> そのパスを優先
        (_, Some(p)) => {
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

    // 2. プロジェクト名の決定
    let project_name = if let Some(n) = name {
        n
    } else {
        // 名前がなければディレクトリ名から推測
        target_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unnamed-project")
            .to_string()
    };

    // 3. 実体とアクションの生成
    let project = Project::new(target_path);
    let args = ProjectCreationArgs { name: project_name };
    let action = CreateAction {
        target: project,
        args,
    };
    let presenter = NewPresenter;

    // 4. 実行
    match action.run(&mut |e| presenter.render_event(e)) {
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

impl CliSpeaker<CreateEvent, CreateError, Loaded<Project, ProjectConfig>> for NewPresenter {
    fn render_event(&self, event: CreateEvent) {
        match event {
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
