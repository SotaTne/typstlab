use anyhow::{Result, anyhow};
use colored::Colorize;
use typstlab_app::{BuildAction, BuildEvent, BuildError, AppContext};
use typstlab_proto::{Action, CliSpeaker};

/// build コマンドのエントリポイント
pub fn run(ctx: AppContext, inputs: Option<Vec<String>>) -> Result<()> {
    // 1. Action の生成 (コンテキストを渡すだけ)
    let action = BuildAction::new(ctx.project, ctx.store, inputs);
    let presenter = BuildPresenter;

    // 2. 実行
    match action.run(&mut |event| presenter.render_event(event)) {
        Ok(out) => {
            presenter.render_result(&out);
            Ok(())
        }
        Err(errors) => {
            for err in &errors {
                presenter.render_error(err);
            }
            Err(anyhow!("Build failed"))
        }
    }
}

struct BuildPresenter;

impl CliSpeaker<BuildEvent, BuildError, ()> for BuildPresenter {
    fn render_event(&self, event: BuildEvent) {
        match event {
            BuildEvent::DiscoveredTargets { count } => {
                println!("{} Found {} target(s) to build.", "📋".blue(), count);
            }
            BuildEvent::DiscoveryStarted { inputs } => {
                if inputs.len() > 1 {
                    println!("{} Resolving {} targets...", "🔍".cyan(), inputs.len());
                }
            }
            BuildEvent::Starting { paper_id } => {
                println!("{} Building {}...", "🔨".cyan(), paper_id.bold());
            }
            BuildEvent::Finished { paper_id, output_path, duration_ms } => {
                println!(
                    "{} {} built successfully! ({}) -> {}", 
                    "✨".green(), 
                    paper_id.bold(), 
                    format!("{}ms", duration_ms).dimmed(),
                    output_path.display().to_string().dimmed()
                );
            }
            _ => {}
        }
    }

    fn render_error(&self, error: &BuildError) {
        match error {
            BuildError::Discovery(errs) => {
                println!("\n{}", "Failed to resolve some targets:".red().bold());
                for err in errs {
                    eprintln!("  {} {}", "•".red(), err);
                }
            }
            BuildError::PaperBuildError { paper_id, error } => {
                eprintln!("{} {} failed: {}", "❌".red(), paper_id.bold(), error);
            }
            _ => {
                eprintln!("{} {}", "❌ ERROR:".red().bold(), error);
            }
        }
    }

    fn render_result(&self, _output: &()) {
        println!("\n{}", "All builds completed successfully!".green().bold());
    }
}
