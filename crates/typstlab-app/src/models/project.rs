use crate::models::paper_scope::PaperScope;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use typstlab_proto::Entity;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub project: ProjectInfo,
    pub typst: TypstInfo,
    #[serde(default)]
    pub structure: StructureConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypstInfo {
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StructureConfig {
    #[serde(default = "default_papers_dir")]
    pub papers_dir: String,
    #[serde(default = "default_dist_dir")]
    pub dist_dir: String,
}

impl Default for StructureConfig {
    fn default() -> Self {
        Self {
            papers_dir: default_papers_dir(),
            dist_dir: default_dist_dir(),
        }
    }
}

fn default_papers_dir() -> String {
    "papers".to_string()
}
fn default_dist_dir() -> String {
    "dist".to_string()
}

pub struct Project {
    pub root: PathBuf,
}

impl Project {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("typstlab.toml")
    }

    pub fn load_config(&self) -> anyhow::Result<ProjectConfig> {
        let content = std::fs::read_to_string(self.config_path())?;
        let config: ProjectConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// プロジェクトの構造設定を取得（ロード失敗時はデフォルト）
    pub fn structure(&self) -> StructureConfig {
        self.load_config().map(|c| c.structure).unwrap_or_default()
    }

    /// 成果物ディレクトリの絶対パス
    pub fn dist_dir(&self) -> PathBuf {
        self.root.join(&self.structure().dist_dir)
    }

    /// プロジェクトの論文スコープ（領土）を取得
    pub fn papers_scope(&self) -> PaperScope {
        PaperScope::new(self.root.clone(), self.structure().papers_dir)
    }
}

impl Entity for Project {
    fn path(&self) -> PathBuf {
        self.root.clone()
    }
}
