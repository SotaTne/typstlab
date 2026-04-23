use anyhow::{Result, anyhow};
use colored::Colorize;
use typstlab_app::{AppContext, BuildAction, BuildError, BuildEvent};
use typstlab_proto::{Action, Artifact, CliSpeaker, Entity};

/// build コマンドのエントリポイント
pub fn run(ctx: AppContext, inputs: Option<Vec<String>>) -> Result<()> {
    let action = BuildAction::new(ctx.loaded_project, ctx.store, inputs);
    let presenter = BuildPresenter;

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
            BuildEvent::Finished {
                artifact,
                duration_ms,
            } => {
                println!(
                    "{} {} built successfully! ({}) -> {}",
                    "✨".green(),
                    artifact.root().display().to_string().bold(),
                    format!("{}ms", duration_ms).dimmed(),
                    artifact.path().display().to_string().dimmed()
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
            BuildError::PaperBuildError(artifact) => {
                eprintln!(
                    "{} {} failed:",
                    "❌".red(),
                    artifact.root().display().to_string().bold()
                );
                // Typst の生のエラー出力をインデント付きで結合して表示
                let raw_error = artifact
                    .error()
                    .unwrap_or_else(|| "Unknown error".to_string());
                let indented_error = raw_error
                    .lines()
                    .map(|line| format!("   {}", line))
                    .collect::<Vec<_>>()
                    .join("\n");

                eprintln!("{}\n", indented_error);
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
