use anyhow::{Result, anyhow};
use colored::Colorize;
use typstlab_app::{BuildAction, BuildEvent, BuildError, AppContext, Paper};
use typstlab_proto::{Action, CliSpeaker};

/// build コマンドのエントリポイント (Skinny Controller)
pub fn run(ctx: AppContext, targets: Option<Vec<Paper>>) -> Result<()> {
    let action = BuildAction::new(ctx.project, ctx.store, targets);
    let presenter = BuildPresenter;

    // 実行と語り (一切の println! を排除)
    match action.run(&mut |event| presenter.render_event(event)) {
        Ok(out) => {
            presenter.render_result(&out);
            Ok(())
        }
        Err(errors) => {
            for err in &errors {
                presenter.render_error(err);
            }
            Err(anyhow!("One or more papers failed to build"))
        }
    }
}

/// build コマンド専用の語り手
struct BuildPresenter;

impl CliSpeaker<BuildEvent, BuildError, ()> for BuildPresenter {
    fn render_event(&self, event: BuildEvent) {
        match event {
            BuildEvent::DiscoveredTargets { count } => {
                println!("{} Found {} target(s) to build.", "📋".blue(), count);
            }
            BuildEvent::Starting { paper_id } => {
                println!("{} Building {}...", "🔨".cyan(), paper_id.bold());
            }
            BuildEvent::Finished { paper_id, output_path } => {
                println!(
                    "{} {} built successfully! -> {}", 
                    "✨".green(), 
                    paper_id.bold(), 
                    output_path.display().to_string().dimmed()
                );
            }
            _ => {}
        }
    }

    fn render_error(&self, error: &BuildError) {
        match error {
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
