use anyhow::{Result, anyhow};
use colored::Colorize;
use std::path::PathBuf;
use typstlab_app::{CreateAction, CreateError, CreateEvent, Project, ProjectCreationArgs};
use typstlab_proto::{Action, CliSpeaker, Entity};

/// new コマンドのエントリポイント
pub fn run(name: String, path: Option<String>) -> Result<()> {
    // 1. 作成場所の決定
    let target_path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        std::env::current_dir()?.join(&name)
    };

    // 2. 実体とアクションの生成
    let project = Project::new(target_path);
    let args = ProjectCreationArgs { name };
    let action = CreateAction {
        target: project,
        args,
    };
    let presenter = NewPresenter;

    // 3. 実行
    match action.run(&mut |e| presenter.render_event(e)) {
        Ok(project) => {
            presenter.render_result(&project);
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

impl CliSpeaker<CreateEvent, CreateError, Project> for NewPresenter {
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

    fn render_result(&self, project: &Project) {
        println!("\n{} Project created successfully!", "🎉".green().bold());
        println!(
            "  Location: {}",
            project.path().display().to_string().cyan()
        );
        println!("\nNext steps:");
        println!("  1. cd {}", project.path().display());
        println!("  2. typstlab build");
    }
}
