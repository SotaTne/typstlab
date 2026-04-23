use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use typstlab_proto::Entity;

/// paper.toml のスキーマ定義
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaperConfig {
    pub paper: PaperInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaperInfo {
    pub title: String,
    #[serde(default = "default_entry_point")]
    pub entry_point: PathBuf,
    /// 成果物のベース名
    #[serde(default = "default_output_name")]
    pub output_name: String,
}

impl Default for PaperConfig {
    fn default() -> Self {
        Self {
            paper: PaperInfo {
                title: "Untitled Paper".to_string(),
                entry_point: default_entry_point(),
                output_name: default_output_name(),
            },
        }
    }
}

fn default_entry_point() -> PathBuf {
    PathBuf::from("main.typ")
}

fn default_output_name() -> String {
    "main".to_string()
}

#[derive(Clone, Debug)]
pub struct Paper {
    pub id: String,
    pub absolute_path: PathBuf,
}

impl Paper {
    pub fn new(id: String, papers_dir: PathBuf) -> Self {
        let absolute_path = papers_dir.join(&id);
        Self { id, absolute_path }
    }

    pub fn config_path(&self) -> PathBuf {
        self.absolute_path.join("paper.toml")
    }

    pub fn load_config(&self) -> anyhow::Result<PaperConfig> {
        let content = std::fs::read_to_string(self.config_path())?;
        let config: PaperConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn config(&self) -> PaperConfig {
        self.load_config().unwrap_or_default()
    }

    /// 成果物のベース名を取得
    pub fn output_base_name(&self) -> String {
        self.config().paper.output_name
    }

    pub fn main_typ_path(&self) -> PathBuf {
        let entry = self.config().paper.entry_point;
        self.absolute_path.join(entry)
    }
}

impl Entity for Paper {
    fn path(&self) -> PathBuf {
        self.absolute_path.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::PaperConfig;
    use std::path::PathBuf;

    #[test]
    fn test_config_deserializes_entry_point_as_pathbuf() {
        let config: PaperConfig = toml::from_str(
            r#"
                [paper]
                title = "Demo"
                entry_point = "src/main.typ"
                output_name = "paper"
            "#,
        )
        .unwrap();

        assert_eq!(
            config.paper.entry_point,
            PathBuf::from("src").join("main.typ")
        );
    }
}
