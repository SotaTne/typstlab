use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use typstlab_proto::{Creatable, Loadable, Loaded, PAPER_SETTING_FILE};

// ... (PaperConfig, PaperInfo 等は既存のまま)

pub struct PaperCreationArgs {
    pub title: String,
}

impl Creatable for Paper {
    type Args = PaperCreationArgs;
    type Config = PaperConfig;
    type Error = PaperError;

    fn initialize(self, args: Self::Args) -> Result<Loaded<Self, Self::Config>, Self::Error> {
        Ok(Loaded {
            actual: self,
            config: PaperConfig {
                paper: PaperInfo {
                    title: args.title,
                    entry_point: default_entry_point(),
                    output_name: default_output_name(),
                },
            },
        })
    }

    fn persist(loaded: &Loaded<Self, Self::Config>) -> Result<(), Self::Error> {
        let toml_content = toml::to_string_pretty(&loaded.config).map_err(PaperError::Serialize)?;

        if !loaded.actual.absolute_path.exists() {
            std::fs::create_dir_all(&loaded.actual.absolute_path)?;
        }

        std::fs::write(loaded.actual.config_path(), toml_content)?;

        // デフォルトの main.typ が存在しない場合は作成
        let main_typ = loaded.main_typ_path();
        if !main_typ.exists() {
            if let Some(parent) = main_typ.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(main_typ, "= Untitled Paper\n\nStart writing here.")?;
        }

        Ok(())
    }
}

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

#[derive(Error, Debug)]
pub enum PaperError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[derive(Clone, Debug)]
pub struct Paper {
    pub id: String,
    pub absolute_path: PathBuf,
}

typstlab_proto::impl_entity! {
    Paper {
        fn path(&self) -> PathBuf {
            self.absolute_path.clone()
        }
    }
}

impl Paper {
    pub fn new(id: String, papers_dir: PathBuf) -> Self {
        let absolute_path = papers_dir.join(&id);
        Self { id, absolute_path }
    }

    pub fn config_path(&self) -> PathBuf {
        self.absolute_path.join(PAPER_SETTING_FILE)
    }
}

impl Loadable for Paper {
    type Config = PaperConfig;
    type Error = PaperError;

    fn load_from_disk(&self) -> Result<Self::Config, Self::Error> {
        let content = std::fs::read_to_string(self.config_path())?;
        let config: PaperConfig = toml::from_str(&content)?;
        Ok(config)
    }
}

pub trait PaperHandle {
    fn output_base_name(&self) -> &str;
    fn main_typ_path(&self) -> PathBuf;
    fn paper_id(&self) -> &str;
}

impl PaperHandle for Loaded<Paper, PaperConfig> {
    fn output_base_name(&self) -> &str {
        &self.config.paper.output_name
    }

    fn main_typ_path(&self) -> PathBuf {
        self.actual
            .absolute_path
            .join(&self.config.paper.entry_point)
    }

    fn paper_id(&self) -> &str {
        &self.actual.id
    }
}

#[cfg(test)]
mod tests {
    use super::{Paper, PaperConfig, PaperError, PaperHandle};
    use std::path::PathBuf;
    use tempfile::TempDir;
    use typstlab_proto::Loadable;

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

    #[test]
    fn test_load_fails_for_invalid_config_instead_of_falling_back_to_default() {
        let temp = TempDir::new().unwrap();
        let paper_root = temp.path().join("p01");
        std::fs::create_dir_all(&paper_root).unwrap();
        std::fs::write(paper_root.join("paper.toml"), "[paper]\ntitle = [").unwrap();

        let paper = Paper {
            id: "p01".to_string(),
            absolute_path: paper_root,
        };

        let error = match paper.load() {
            Ok(_) => panic!("expected invalid paper config to fail loading"),
            Err(error) => error,
        };

        assert!(matches!(error, PaperError::Parse(_)));
    }

    #[test]
    fn test_loaded_paper_exposes_output_and_entry_paths() {
        let temp = TempDir::new().unwrap();
        let paper_root = temp.path().join("p01");
        std::fs::create_dir_all(&paper_root).unwrap();
        std::fs::write(
            paper_root.join("paper.toml"),
            r#"
                [paper]
                title = "Demo"
                entry_point = "src/main.typ"
                output_name = "camera-ready"
            "#,
        )
        .unwrap();

        let paper = Paper {
            id: "p01".to_string(),
            absolute_path: paper_root.clone(),
        };

        let loaded_paper = paper.load().unwrap();

        assert_eq!(loaded_paper.paper_id(), "p01");
        assert_eq!(loaded_paper.output_base_name(), "camera-ready");
        assert_eq!(
            loaded_paper.main_typ_path(),
            paper_root.join("src").join("main.typ")
        );
    }
}
