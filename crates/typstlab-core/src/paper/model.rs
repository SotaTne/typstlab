use serde::{Deserialize, Serialize};

/// paper.toml schema - 個別 paper のメタ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperConfig {
    pub paper: PaperMeta,
    #[serde(default)]
    pub layout: LayoutConfig,
    pub output: OutputConfig,
    #[serde(default)]
    pub build: PaperBuildConfig,
    #[serde(default)]
    pub refs: Option<RefsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperMeta {
    pub id: String,
    pub title: String,
    pub language: String,
    pub date: String,
    #[serde(default)]
    pub authors: Vec<Author>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub affiliation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    #[serde(default = "default_layout_name")]
    pub name: String,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
        }
    }
}

fn default_layout_name() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperBuildConfig {
    #[serde(default = "default_targets")]
    pub targets: Vec<String>,
}

impl Default for PaperBuildConfig {
    fn default() -> Self {
        Self {
            targets: vec!["pdf".to_string()],
        }
    }
}

fn default_targets() -> Vec<String> {
    vec!["pdf".to_string()]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefsConfig {
    #[serde(default)]
    pub sets: Vec<String>,
}

impl PaperConfig {
    /// paper.toml を読み込む
    pub fn from_file(path: impl AsRef<std::path::Path>) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| crate::error::TypstlabError::ConfigParseError(e.to_string()))?;

        toml::from_str(&content).map_err(|e| {
            crate::error::TypstlabError::PaperConfigInvalid {
                paper_id: "unknown".to_string(),
                reason: e.to_string(),
            }
        })
    }

    /// paper.toml に書き込む
    pub fn to_file(&self, path: impl AsRef<std::path::Path>) -> crate::error::Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::error::TypstlabError::ConfigParseError(e.to_string()))?;

        std::fs::write(path.as_ref(), content)
            .map_err(|e| crate::error::TypstlabError::IoError(e))?;

        Ok(())
    }

    /// paper.id とディレクトリ名が一致するか検証
    pub fn validate_id(&self, dir_name: &str) -> crate::error::Result<()> {
        if self.paper.id != dir_name {
            return Err(crate::error::TypstlabError::PaperIdMismatch {
                toml_id: self.paper.id.clone(),
                dir_name: dir_name.to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_paper() {
        let toml = r#"
[paper]
id = "report"
title = "My Report"
language = "en"
date = "2026-01-05"

[output]
name = "report"
"#;
        let config: PaperConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.paper.id, "report");
        assert_eq!(config.paper.title, "My Report");
        assert_eq!(config.layout.name, "default");
        assert_eq!(config.build.targets, vec!["pdf"]);
    }

    #[test]
    fn test_parse_full_paper() {
        let toml = r#"
[paper]
id = "report"
title = "My Research Report"
language = "en"
date = "2026-01-05"

[[paper.authors]]
name = "Alice"
email = "alice@example.com"
affiliation = "University"

[[paper.authors]]
name = "Bob"
email = "bob@example.com"
affiliation = "Company"

[layout]
name = "ieee"

[output]
name = "report"

[build]
targets = ["pdf"]

[refs]
sets = ["core", "report-2026q1"]
"#;
        let config: PaperConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.paper.id, "report");
        assert_eq!(config.paper.authors.len(), 2);
        assert_eq!(config.layout.name, "ieee");
        assert_eq!(config.refs.as_ref().unwrap().sets, vec!["core", "report-2026q1"]);
    }

    #[test]
    fn test_validate_id_match() {
        let toml = r#"
[paper]
id = "report"
title = "My Report"
language = "en"
date = "2026-01-05"

[output]
name = "report"
"#;
        let config: PaperConfig = toml::from_str(toml).unwrap();
        assert!(config.validate_id("report").is_ok());
    }

    #[test]
    fn test_validate_id_mismatch() {
        let toml = r#"
[paper]
id = "report"
title = "My Report"
language = "en"
date = "2026-01-05"

[output]
name = "report"
"#;
        let config: PaperConfig = toml::from_str(toml).unwrap();
        assert!(config.validate_id("thesis").is_err());
    }
}
