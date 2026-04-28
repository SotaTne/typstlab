use anyhow::{Result, anyhow};
use colored::Colorize;
use typstlab_app::{
    AppContext, BuildAction, BuildError, BuildEvent, BuildFormat, BuildWarning, DistObject,
};
use typstlab_proto::{Action, AppEvent, Artifact, CliSpeaker, Entity};

/// build コマンドのエントリポイント
pub fn run(
    ctx: AppContext,
    inputs: Option<Vec<String>>,
    format: BuildFormat,
    verbose: bool,
) -> Result<()> {
    use typstlab_base::driver::TypstDriver;
    let driver = TypstDriver::new(ctx.typst.path());
    let action = BuildAction::new(ctx.loaded_project, driver, inputs, format);
    let presenter = BuildPresenter;
    let mut warning_seen = false;

    match action.run(
        &mut |event| {
            if event.visible_in_cli(verbose) {
                presenter.render_event(event);
            }
        },
        &mut |warning| {
            warning_seen = true;
            presenter.render_warning(warning);
        },
    ) {
        Ok(out) => {
            if !warning_seen {
                presenter.render_result(&out);
            }
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

impl CliSpeaker for BuildPresenter {
    type Event = BuildEvent;
    type Warning = BuildWarning;
    type Error = BuildError;
    type Output = Vec<DistObject>;

    fn render_event(&self, event: AppEvent<BuildEvent>) {
        match event.payload {
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

    fn render_warning(&self, warning: BuildWarning) {
        match warning {
            BuildWarning::NoTargetsFound => {
                eprintln!("{} No papers found to build.", "⚠ WARNING:".yellow().bold());
            }
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
                let artifact_error = artifact.error();
                let raw_error = artifact_error
                    .as_deref()
                    .unwrap_or("artifact reported no error message");
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

    fn render_result(&self, output: &Vec<DistObject>) {
        println!("\n{}", "All builds completed successfully!".green().bold());
        for dist in output {
            println!("  📄 {}", dist.paper_id.bold());
            if let Some(pdf) = &dist.pdf {
                println!("    {} {}", "PDF:".cyan(), pdf.display());
            }
            if let Some(pngs) = &dist.png {
                println!("    {} {} pages", "PNG:".cyan(), pngs.len());
            }
            if let Some(svgs) = &dist.svg {
                println!("    {} {} pages", "SVG:".cyan(), svgs.len());
            }
            if let Some(html) = &dist.html {
                println!("    {} {}", "HTML:".cyan(), html.display());
            }
        }
    }
}
