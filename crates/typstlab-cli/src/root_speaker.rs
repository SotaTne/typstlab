use colored::Colorize;
use anyhow::{Result, anyhow};
use std::path::PathBuf;
use typstlab_app::ManagedStore;

/// アプリケーション全体の起動プロセスを語る Speaker
pub struct RootSpeaker;

impl RootSpeaker {
    pub fn new() -> Self {
        Self
    }

    /// ストア（キャッシュ）の準備を実況しながら実行
    pub fn prepare_store(&self) -> Result<ManagedStore> {
        dirs::cache_dir()
            .map(|d| {
                let path = d.join("typstlab");
                ManagedStore::new(path)
            })
            .ok_or_else(|| {
                let err = anyhow!("Could not find system cache directory");
                self.render_critical_error(&err);
                err
            })
    }

    /// プロジェクトの特定を実況
    pub fn identify_project(&self) -> Result<PathBuf> {
        std::env::current_dir().map_err(|e| {
            let err = anyhow!("Failed to identify current directory: {}", e);
            self.render_critical_error(&err);
            err
        })
    }

    /// 致命的な起動エラーを語る
    pub fn render_critical_error(&self, error: &anyhow::Error) {
        eprintln!("\n{} {}", "💥 CRITICAL ERROR:".red().bold(), error);
        eprintln!("{}\n", "Please check your environment and try again.".dimmed());
    }
}
