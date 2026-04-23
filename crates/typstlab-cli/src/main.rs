mod commands;

use clap::{Parser, Subcommand};
use colored::Colorize;
use thiserror::Error;
use typstlab_app::{
    BootstrapAction, BootstrapEvent, BootstrapError, LoadEvent,
};
use typstlab_proto::{Action, CliSpeaker};

#[derive(Parser, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Clone)]
pub enum Commands {
    /// Build papers
    Build {
        /// Optional paper IDs or paths to build (if omitted, builds all)
        papers: Vec<String>,
    },
    /// Create a new project
    New {
        /// Project name (optional, defaults to current directory name)
        name: Option<String>,
        /// Optional path to create the project (defaults to .)
        path: Option<String>,
    },
}

pub struct CliAction {
    pub cli: Cli,
}

#[derive(Debug)]
pub enum CliEvent {
    Bootstrap(BootstrapEvent),
}

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Initialization failed: {0}")]
    Bootstrap(#[from] BootstrapError),
    #[error("Command failed: {0}")]
    Command(String),
    #[error("System error: {0}")]
    System(String),
}

impl Action<(), CliEvent, CliError> for CliAction {
    fn run(self, monitor: &mut dyn FnMut(CliEvent)) -> Result<(), Vec<CliError>> {
        match &self.cli.command {
            Commands::New { name, path } => {
                commands::new::run(name.clone(), path.clone())
                    .map_err(|e| vec![CliError::Command(e.to_string())])?;
                return Ok(());
            }
            
            Commands::Build { papers } => {
                let project_root = std::env::current_dir()
                    .map_err(|e| vec![CliError::System(format!("Could not identify current directory: {}", e))])?;
                    
                let cache_root = dirs::cache_dir()
                    .ok_or_else(|| vec![CliError::System("Could not find cache directory".to_string())])?
                    .join("typstlab");

                let bootstrap = BootstrapAction { project_root, cache_root };
                let ctx = bootstrap.run(&mut |e| monitor(CliEvent::Bootstrap(e)))
                    .map_err(|errors| errors.into_iter().map(CliError::Bootstrap).collect::<Vec<_>>())?;

                let inputs = if papers.is_empty() { None } else { Some(papers.clone()) };
                commands::build::run(ctx, inputs)
                    .map_err(|e| vec![CliError::Command(e.to_string())])?;
            }
        }

        Ok(())
    }
}

struct RootPresenter;

impl CliSpeaker<CliEvent, CliError, ()> for RootPresenter {
    fn render_event(&self, event: CliEvent) {
        match event {
            CliEvent::Bootstrap(e) => {
                use typstlab_app::ResolveEvent;
                match e {
                    BootstrapEvent::ProjectLoading(le) => {
                        if let LoadEvent::Started = le {
                            println!("{} Loading project configuration...", "⏳".cyan());
                        }
                    }
                    BootstrapEvent::ProjectReady { name } => {
                        println!("{} Project: {}", "📁".blue(), name.bold());
                    }
                    BootstrapEvent::ResolvingTypst { version, event } => {
                        match event {
                            ResolveEvent::CacheMiss => {
                                println!("{} Typst {} not found, preparing to download...", "📥".yellow(), version);
                            }
                            _ => {}
                        }
                    }
                    BootstrapEvent::Ready => {
                        println!("{} Environment ready.", "✅".green());
                    }
                    _ => {}
                }
            }
        }
    }

    fn render_error(&self, error: &CliError) {
        eprintln!("\n{} {}", "💥 ERROR:".red().bold(), error);
    }

    fn render_result(&self, _output: &()) {}
}

fn main() {
    let cli = Cli::parse();
    let presenter = RootPresenter;
    let action = CliAction { cli };

    match action.run(&mut |e| presenter.render_event(e)) {
        Ok(out) => presenter.render_result(&out),
        Err(errors) => {
            for err in errors {
                presenter.render_error(&err);
            }
            std::process::exit(1);
        }
    }
}
