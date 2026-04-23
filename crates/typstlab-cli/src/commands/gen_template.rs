use anyhow::{Result, anyhow};
use colored::Colorize;
use typstlab_app::AppContext;
use typstlab_app::actions::gen_template::{GenTemplateAction, GenTemplateError, GenTemplateEvent};
use typstlab_proto::{Action, CliSpeaker};

/// `gen template` コマンドのエントリポイント
pub fn run(ctx: AppContext, id: String) -> Result<()> {
    let action = GenTemplateAction {
        project: ctx.loaded_project,
        template_id: id.clone(),
    };

    let presenter = GenTemplatePresenter { target_id: id };

    match action.run(&mut |event| presenter.render_event(event), &mut |_| {}) {
        Ok(_) => {
            presenter.render_result(&());
            Ok(())
        }
        Err(errors) => {
            for err in &errors {
                presenter.render_error(err);
            }
            Err(anyhow!("Failed to generate template"))
        }
    }
}

// --- Presenter (語り手) ---

struct GenTemplatePresenter {
    target_id: String,
}

impl CliSpeaker<GenTemplateEvent, (), GenTemplateError, ()> for GenTemplatePresenter {
    fn render_event(&self, event: GenTemplateEvent) {
        match event {
            GenTemplateEvent::CreatingTemplate(_) => {
                println!(
                    "{} Creating new local template '{}'...",
                    "🐣".cyan(),
                    self.target_id.bold()
                );
            }
            GenTemplateEvent::TemplateReady { path } => {
                println!(
                    "{} Scaffolded template at {}",
                    "📦".blue(),
                    path.display().to_string().dimmed()
                );
            }
        }
    }

    fn render_warning(&self, _warning: ()) {}

    fn render_error(&self, error: &GenTemplateError) {
        eprintln!("{} {}", "❌ ERROR:".red().bold(), error);
    }

    fn render_result(&self, _output: &()) {
        println!(
            "\n{} Template '{}' generated successfully!",
            "🎉".green().bold(),
            self.target_id
        );
        println!(
            "  You can now use it with: typstlab gen paper <id> -t {}",
            self.target_id
        );
    }
}
