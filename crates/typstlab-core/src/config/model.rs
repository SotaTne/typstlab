use serde::{Deserialize, Serialize};

/// typstlab.toml schema - プロジェクト全体の規約
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub project: ProjectConfig,
    pub typst: TypstConfig,
    #[serde(default)]
    pub tools: ToolsConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default)]
    pub watch: WatchConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub init_date: String,
    #[serde(default)]
    pub default_author: Option<AuthorConfig>,
    #[serde(default)]
    pub default_layout: Option<DefaultLayoutConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorConfig {
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub affiliation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultLayoutConfig {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypstConfig {
    /// 完全一致要求
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsConfig {
    #[serde(default)]
    pub uv: Option<ToolRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequirement {
    pub required: bool,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "default_network_policy")]
    pub policy: NetworkPolicy,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            policy: NetworkPolicy::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NetworkPolicy {
    Auto,
    Never,
}

fn default_network_policy() -> NetworkPolicy {
    NetworkPolicy::Auto
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    #[serde(default = "default_parallel")]
    pub parallel: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self { parallel: true }
    }
}

fn default_parallel() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    #[serde(default = "default_ignore")]
    pub ignore: Vec<String>,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 500,
            ignore: vec![
                "*.tmp".to_string(),
                ".DS_Store".to_string(),
                "*.swp".to_string(),
            ],
        }
    }
}

fn default_debounce_ms() -> u64 {
    500
}

fn default_ignore() -> Vec<String> {
    vec![
        "*.tmp".to_string(),
        ".DS_Store".to_string(),
        "*.swp".to_string(),
    ]
}

impl Config {
    /// typstlab.toml を読み込む
    pub fn from_file(path: impl AsRef<std::path::Path>) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| crate::error::TypstlabError::ConfigParseError(e.to_string()))?;

        toml::from_str(&content)
            .map_err(|e| crate::error::TypstlabError::ProjectConfigInvalid(e.to_string()))
    }

    /// typstlab.toml に書き込む
    pub fn to_file(&self, path: impl AsRef<std::path::Path>) -> crate::error::Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::error::TypstlabError::ConfigParseError(e.to_string()))?;

        std::fs::write(path.as_ref(), content).map_err(crate::error::TypstlabError::IoError)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
[project]
name = "my-research"
init_date = "2026-01-05"

[typst]
version = "0.13.1"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.project.name, "my-research");
        assert_eq!(config.typst.version, "0.13.1");
        assert_eq!(config.network.policy, NetworkPolicy::Auto);
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
[project]
name = "my-research"
init_date = "2026-01-05"

[project.default_author]
name = "Alice"
email = "alice@example.com"
affiliation = "University"

[project.default_layout]
name = "default"

[typst]
version = "0.13.1"

[tools.uv]
required = true

[network]
policy = "never"

[build]
parallel = false

[watch]
debounce_ms = 1000
ignore = ["*.bak"]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.project.name, "my-research");
        assert_eq!(config.network.policy, NetworkPolicy::Never);
        assert!(!config.build.parallel);
        assert_eq!(config.watch.debounce_ms, 1000);
    }
}
