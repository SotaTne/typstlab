use anyhow::{Result, anyhow};
use colored::Colorize;
use typstlab_app::AppContext;
use typstlab_app::actions::gen_paper::{GenPaperAction, GenPaperError, GenPaperEvent};
use typstlab_base::driver::TypstDriver;
use typstlab_proto::{Action, AppEvent, CliSpeaker};

/// `gen paper` コマンドのエントリポイント
pub fn run(ctx: AppContext, id: String, template: Option<String>, verbose: bool) -> Result<()> {
    // 1. Typst ドライバーの解決 (Build と同様に Project のバージョンを使用)
    let typst = ctx.typst;

    use typstlab_proto::Entity;
    let driver = TypstDriver::new(typst.path());

    // 2. Action の準備
    let action = GenPaperAction {
        project: ctx.loaded_project,
        paper_id: id.clone(),
        template_input: template,
        typst_driver: driver,
    };

    let presenter = GenPaperPresenter { target_id: id };

    // 3. 実行と実況
    match action.run(
        &mut |event| {
            if event.visible_in_cli(verbose) {
                presenter.render_event(event);
            }
        },
        &mut |_| {}, // Warningはない
    ) {
        Ok(_) => {
            presenter.render_result(&());
            Ok(())
        }
        Err(errors) => {
            for err in &errors {
                presenter.render_error(err);
            }
            Err(anyhow!("Failed to generate paper"))
        }
    }
}

// --- Presenter (語り手) ---

struct GenPaperPresenter {
    target_id: String,
}

impl CliSpeaker for GenPaperPresenter {
    type Event = GenPaperEvent;
    type Warning = ();
    type Error = GenPaperError;
    type Output = ();

    fn render_event(&self, event: AppEvent<GenPaperEvent>) {
        match event.payload {
            GenPaperEvent::ResolvingTemplate { id } => {
                println!("{} Resolving template '{}'...", "🔍".cyan(), id.bold());
            }
            GenPaperEvent::ResolvedLocal { path } => {
                println!(
                    "{} Found local template at {}",
                    "✅".green(),
                    path.display().to_string().dimmed()
                );
            }
            GenPaperEvent::FallbackToInit { template_input } => {
                println!(
                    "{} Local template not found. Delegating to `typst init {}`...",
                    "🌐".yellow(),
                    template_input.bold()
                );
            }
            GenPaperEvent::CreatingPaper(create_event) => {
                use typstlab_app::actions::create::CreateEvent;
                match create_event {
                    CreateEvent::Initializing => {
                        println!(
                            "{} Initializing paper '{}'...",
                            "🐣".cyan(),
                            self.target_id.bold()
                        );
                    }
                    CreateEvent::Persisting => {
                        println!("{} Writing configuration...", "📝".cyan());
                    }
                    CreateEvent::Completed => {}
                }
            }
            GenPaperEvent::PaperReady { path } => {
                println!(
                    "{} Scaffolded paper at {}",
                    "📦".blue(),
                    path.display().to_string().dimmed()
                );
            }
        }
    }

    fn render_warning(&self, _warning: ()) {}

    fn render_error(&self, error: &GenPaperError) {
        match error {
            GenPaperError::TemplateOrInitFailed(e) => {
                eprintln!(
                    "{} Failed to apply template or run `typst init`:\n  {}",
                    "❌".red().bold(),
                    e.dimmed()
                );
            }
            _ => {
                eprintln!("{} {}", "❌ ERROR:".red().bold(), error);
            }
        }
    }

    fn render_result(&self, _output: &()) {
        println!(
            "\n{} Paper '{}' generated successfully!",
            "🎉".green().bold(),
            self.target_id
        );
    }
}
