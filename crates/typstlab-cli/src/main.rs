mod commands;

use clap::{Parser, Subcommand};
use colored::Colorize;
use thiserror::Error;
use typstlab_app::{
    BootstrapAction, BootstrapEvent, BootstrapError, 
    DiscoveryAction, DiscoveryError,
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
}

// ============================================================================
// CLI 全体を一つの Action として実体化
// ============================================================================

pub struct CliAction {
    pub cli: Cli,
}

#[derive(Debug)]
pub enum CliEvent {
    Bootstrap(BootstrapEvent),
    // 今後、他のグローバルイベントがあれば追加
}

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Initialization failed: {0}")]
    Bootstrap(#[from] BootstrapError),
    #[error("Target discovery failed: {0:?}")]
    Discovery(Vec<DiscoveryError>),
    #[error("Command failed: {0}")]
    Command(String),
    #[error("System error: {0}")]
    System(String),
}

impl Action<(), CliEvent, CliError> for CliAction {
    fn run(&self, monitor: &mut dyn FnMut(CliEvent)) -> Result<(), Vec<CliError>> {
        // 1. 環境情報の取得
        let project_root = std::env::current_dir()
            .map_err(|e| vec![CliError::System(format!("Could not identify current directory: {}", e))])?;
            
        let cache_root = dirs::cache_dir()
            .ok_or_else(|| vec![CliError::System("Could not find cache directory".to_string())])?
            .join("typstlab");

        // 2. Bootstrap 実行
        let bootstrap = BootstrapAction { project_root, cache_root };
        let ctx = bootstrap.run(&mut |e| monitor(CliEvent::Bootstrap(e)))
            .map_err(|errors| errors.into_iter().map(CliError::Bootstrap).collect::<Vec<_>>())?;

        // 3. コマンドへの振り分け
        match &self.cli.command {
            Commands::Build { papers } => {
                // ターゲットの特定 (DiscoveryAction を使用)
                let targets = if !papers.is_empty() {
                    let discovery = DiscoveryAction {
                        scope: ctx.project.papers_scope(),
                        inputs: papers.clone(),
                    };
                    // 特定に失敗してもエラーを集約して Speaker に渡す
                    let resolved = discovery.run(&mut |_| {})
                        .map_err(|errs| vec![CliError::Discovery(errs)])?;
                    Some(resolved)
                } else {
                    None
                };

                commands::build::run(ctx, targets)
                    .map_err(|e| vec![CliError::Command(e.to_string())])?;
            }
        }

        Ok(())
    }
}

// ============================================================================
// CLI 全体の語り手 (Root Presenter)
// ============================================================================

struct RootPresenter;

impl CliSpeaker<CliEvent, CliError, ()> for RootPresenter {
    fn render_event(&self, event: CliEvent) {
        match event {
            CliEvent::Bootstrap(e) => {
                use typstlab_app::ResolveEvent;
                match e {
                    BootstrapEvent::ProjectReady { name } => {
                        println!("{} Project: {}", "📁".blue(), name.bold());
                    }
                    BootstrapEvent::ResolvingTypst { version, event } => {
                        if let ResolveEvent::CacheMiss = event {
                            println!("{} Typst {} not found, preparing to download...", "📥".yellow(), version);
                        }
                    }
                    BootstrapEvent::ResolvingDocs { version, event } => {
                        if let ResolveEvent::CacheMiss = event {
                            println!("{} Documentation for {} not found, syncing...", "📥".yellow(), version);
                        }
                    }
                    BootstrapEvent::Ready => {
                        println!("{} Environment initialized.", "✅".green());
                    }
                    _ => {}
                }
            }
        }
    }

    fn render_error(&self, error: &CliError) {
        match error {
            CliError::Discovery(errs) => {
                println!("\n{}", "Failed to resolve some targets:".red().bold());
                for err in errs {
                    eprintln!("  {} {}", "•".red(), err);
                }
            }
            _ => {
                eprintln!("\n{} {}", "💥 ERROR:".red().bold(), error);
            }
        }
    }

    fn render_result(&self, _output: &()) {
        println!("\n{}", "Process completed successfully!".green().bold());
    }
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
