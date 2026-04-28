mod commands;
mod utils;

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use thiserror::Error;
use typstlab_app::{BootstrapError, BootstrapEvent, LoadEvent};
use typstlab_proto::{Action, AppEvent, CliSpeaker};
use utils::bootstrap_context;

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
    Build {
        /// Optional paper IDs or paths to build (if omitted, builds all)
        papers: Vec<String>,
        /// Output PDF document (default if no other formats are specified)
        #[arg(long)]
        pdf: bool,
        /// Output PNG images for each page
        #[arg(long)]
        png: bool,
        /// Output SVG images for each page
        #[arg(long)]
        svg: bool,
        /// Output HTML document
        #[arg(long)]
        html: bool,
    },
    /// Show project status
    Status,
    /// Create a new project
    New {
        /// Project name (optional, defaults to current directory name)
        name: Option<String>,
        /// Optional path to create the project (defaults to .)
        path: Option<String>,
    },
    /// Generate a new paper or template
    Gen {
        #[command(subcommand)]
        subcommand: GenCommands,
    },
    /// Run the MCP server
    Mcp {
        #[command(subcommand)]
        subcommand: McpCommands,
    },
}

#[derive(Subcommand, Clone)]
pub enum GenCommands {
    /// Create a new paper
    Paper {
        /// Paper ID (directory name)
        id: String,
        /// Optional template name or Typst package
        #[arg(short, long)]
        template: Option<String>,
    },
    /// Create a new template
    Template {
        /// Template ID (directory name)
        id: String,
    },
}

#[derive(Subcommand, Clone)]
pub enum McpCommands {
    /// Run the MCP server over stdio for a project root
    Stdio {
        /// Project root containing typstlab.toml
        root: PathBuf,
    },
}

pub struct CliAction {
    pub cli: Cli,
}

#[derive(Debug, Clone)]
pub enum CliEvent {
    Bootstrap(BootstrapEvent),
}

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Initialization failed: {0}")]
    Bootstrap(#[from] BootstrapError),
    #[error("Command failed: {0}")]
    Command(String),
    #[error(
        "Project root not found from '{start}': no {config_file} found in current or parent directories"
    )]
    ProjectRootNotFound {
        start: PathBuf,
        config_file: &'static str,
    },
    #[error("System error: {0}")]
    System(String),
}

impl Action for CliAction {
    type Output = ();
    type Event = CliEvent;
    type Warning = ();
    type Error = CliError;

    fn run(
        self,
        monitor: &mut dyn FnMut(AppEvent<CliEvent>),
        _warning: &mut dyn FnMut(()),
    ) -> Result<Self::Output, Vec<Self::Error>> {
        match &self.cli.command {
            Commands::New { name, path } => {
                commands::new::run(name.clone(), path.clone(), self.cli.verbose)
                    .map_err(|e| vec![CliError::Command(e.to_string())])?;
                return Ok(());
            }

            Commands::Build {
                papers,
                pdf,
                png,
                svg,
                html,
            } => {
                let ctx = bootstrap_context(&mut |e| {
                    monitor(e.map_payload(CliEvent::Bootstrap));
                })
                .map_err(|error| vec![error])?;

                let inputs = if papers.is_empty() {
                    None
                } else {
                    Some(papers.clone())
                };

                let mut actual_pdf = *pdf;
                if !actual_pdf && !png && !svg && !html {
                    actual_pdf = true;
                }

                let format = typstlab_app::BuildFormat {
                    pdf: actual_pdf,
                    png: *png,
                    svg: *svg,
                    html: *html,
                };
                commands::build::run(ctx, inputs, format, self.cli.verbose)
                    .map_err(|e| vec![CliError::Command(e.to_string())])?;
            }

            Commands::Status => {
                let ctx = bootstrap_context(&mut |e| {
                    monitor(e.map_payload(CliEvent::Bootstrap));
                })
                .map_err(|error| vec![error])?;

                commands::status::run(ctx, self.cli.verbose)
                    .map_err(|e| vec![CliError::Command(e.to_string())])?;
            }

            Commands::Gen { subcommand } => {
                let ctx = bootstrap_context(&mut |e| {
                    monitor(e.map_payload(CliEvent::Bootstrap));
                })
                .map_err(|error| vec![error])?;

                match subcommand {
                    GenCommands::Paper { id, template } => {
                        commands::gen_paper::run(
                            ctx,
                            id.clone(),
                            template.clone(),
                            self.cli.verbose,
                        )
                        .map_err(|e| vec![CliError::Command(e.to_string())])?;
                    }
                    GenCommands::Template { id } => {
                        commands::gen_template::run(ctx, id.clone(), self.cli.verbose)
                            .map_err(|e| vec![CliError::Command(e.to_string())])?;
                    }
                }
            }

            Commands::Mcp { subcommand } => match subcommand {
                McpCommands::Stdio { root } => {
                    commands::mcp::run_stdio(root.clone())
                        .map_err(|e| vec![CliError::Command(e.to_string())])?;
                }
            },
        }

        Ok(())
    }
}

struct RootPresenter;

impl CliSpeaker for RootPresenter {
    type Event = CliEvent;
    type Warning = ();
    type Error = CliError;
    type Output = ();

    fn render_event(&self, event: AppEvent<CliEvent>) {
        match event.payload {
            CliEvent::Bootstrap(e) => {
                use typstlab_app::ResolveEvent;
                match e {
                    BootstrapEvent::ProjectLoading(LoadEvent::Started) => {
                        println!("{} Loading project configuration...", "⏳".cyan());
                    }
                    BootstrapEvent::ProjectReady { name } => {
                        println!("{} Project: {}", "📁".blue(), name.bold());
                    }
                    BootstrapEvent::ResolvingTypst {
                        version,
                        event: ResolveEvent::CacheMiss,
                    } => {
                        println!(
                            "{} Typst {} not found, preparing to download...",
                            "📥".yellow(),
                            version
                        );
                    }
                    BootstrapEvent::Ready => {
                        println!("{} Environment ready.", "✅".green());
                    }
                    _ => {}
                }
            }
        }
    }

    fn render_warning(&self, _warning: ()) {}

    fn render_error(&self, error: &CliError) {
        eprintln!("\n{} {}", "💥 ERROR:".red().bold(), error);
    }

    fn render_result(&self, _output: &()) {}
}

fn main() {
    let cli = Cli::parse();
    let verbose = cli.verbose;
    let presenter = RootPresenter;
    let action = CliAction { cli };

    match action.run(
        &mut |event| {
            if event.visible_in_cli(verbose) {
                presenter.render_event(event);
            }
        },
        &mut |_| {},
    ) {
        Ok(out) => presenter.render_result(&out),
        Err(errors) => {
            for err in errors {
                presenter.render_error(&err);
            }
            std::process::exit(1);
        }
    }
}
