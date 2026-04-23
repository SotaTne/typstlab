use anyhow::{Result, anyhow};
use colored::Colorize;
use std::path::Path;
use typstlab_app::{CreateAction, CreateEvent, CreateError, Project, ProjectCreationArgs};
use typstlab_proto::{Action, CliSpeaker, Entity};

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
            if p.is_absolute() { p.to_path_buf() } else { current_dir.join(p) }
        }
    };

    // 2. プロジェクト名の決定
    let project_name = if let Some(n) = name {
        n
    } else {
        // 名前がなければディレクトリ名から推測
        target_path.file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unnamed-project")
            .to_string()
    };

    // 3. 実体とアクションの生成
    let project = Project::new(target_path);
    let args = ProjectCreationArgs { name: project_name };
    let action = CreateAction { target: project, args };
    let presenter = NewPresenter;

    // 4. 実行
    match action.run(&mut |e| presenter.render_event(e)) {
        Ok(project) => {
            // パス移動や . を解決した「綺麗な絶対パス」を持つ実体を再生成して結果を表示
            let clean_root = std::fs::canonicalize(project.path())
                .unwrap_or_else(|_| project.path());
            
            let clean_project = Project {
                root: clean_root,
                config: project.config,
            };
            
            presenter.render_result(&clean_project);
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
        // 解決済みの綺麗なパスを表示
        println!("  Location: {}", project.path().display().to_string().cyan());
        println!("\nNext steps:");
        println!("  1. cd {}", project.path().display());
        println!("  2. typstlab build");
    }
}
