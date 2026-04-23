use std::path::{Path, PathBuf};
use typstlab_proto::Entity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaperConfig {
    pub paper: PaperInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaperInfo {
    pub title: String,
    #[serde(default = "default_entry_point")]
    pub entry_point: String,
}

impl Default for PaperConfig {
    fn default() -> Self {
        Self {
            paper: PaperInfo {
                title: "Untitled Paper".to_string(),
                entry_point: default_entry_point(),
            },
        }
    }
}

fn default_entry_point() -> String {
    "main.typ".to_string()
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

    pub fn main_typ_path(&self) -> PathBuf {
        let entry = self.config().paper.entry_point;
        self.absolute_path.join(Path::new(&entry))
    }
}

impl Entity for Paper {
    fn path(&self) -> PathBuf {
        self.absolute_path.clone()
    }
}
