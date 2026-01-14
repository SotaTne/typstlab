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
    /// Theme name (corresponds to directory in layouts/)
    #[serde(default = "default_layout_theme")]
    pub theme: String,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
        }
    }
}

fn default_layout_theme() -> String {
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

    /// Main file to compile (default: "main.typ")
    #[serde(default = "default_main_file")]
    pub main_file: String,

    /// Root directory for Typst --root option (optional)
    /// If specified, this becomes the base for absolute path resolution
    pub root: Option<String>,
}

impl Default for PaperBuildConfig {
    fn default() -> Self {
        Self {
            targets: vec!["pdf".to_string()],
            main_file: "main.typ".to_string(),
            root: None,
        }
    }
}

fn default_targets() -> Vec<String> {
    vec!["pdf".to_string()]
}

fn default_main_file() -> String {
    "main.typ".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefsConfig {
    #[serde(default)]
    pub sets: Vec<String>,
}

/// Represents a paper in a typstlab project
#[derive(Debug)]
pub struct Paper {
    root: std::path::PathBuf,
    config: PaperConfig,
}

impl Paper {
    /// Load a paper from a directory
    ///
    /// Reads `paper.toml` from the directory and validates that the paper ID
    /// matches the directory name.
    pub fn load(root: std::path::PathBuf) -> crate::error::Result<Self> {
        let config = PaperConfig::from_file(root.join("paper.toml"))?;

        // Validate that paper ID matches directory name
        if let Some(dir_name) = root.file_name().and_then(|n| n.to_str()) {
            config.validate_id(dir_name)?;
        }

        Ok(Self { root, config })
    }

    /// Get the root directory path
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    /// Get the paper configuration
    pub fn config(&self) -> &PaperConfig {
        &self.config
    }

    /// Get the paper ID
    pub fn id(&self) -> &str {
        &self.config.paper.id
    }

    /// Get main file path (relative to paper root)
    ///
    /// This combines the build.root directory (if specified) with the build.main_file.
    ///
    /// # Examples
    ///
    /// - If build.root is None and build.main_file is "main.typ": returns "main.typ"
    /// - If build.root is "src" and build.main_file is "index.typ": returns "src/index.typ"
    pub fn main_file_path(&self) -> std::path::PathBuf {
        use std::path::Path;

        if let Some(root) = &self.config.build.root {
            Path::new(root).join(&self.config.build.main_file)
        } else {
            std::path::PathBuf::from(&self.config.build.main_file)
        }
    }

    /// Get absolute main file path
    ///
    /// Returns the full path to the main file by joining the paper root
    /// with the result of main_file_path().
    pub fn absolute_main_file_path(&self) -> std::path::PathBuf {
        self.root.join(self.main_file_path())
    }

    /// Get root directory for Typst --root option (absolute)
    ///
    /// Returns the absolute path to the root directory if build.root is specified.
    /// This is used as the argument to Typst's --root option.
    ///
    /// # Returns
    ///
    /// - Some(path) if build.root is specified
    /// - None if build.root is not specified
    pub fn typst_root_dir(&self) -> Option<std::path::PathBuf> {
        self.config.build.root.as_ref().map(|r| self.root.join(r))
    }

    /// Check if main file exists
    ///
    /// Uses absolute_main_file_path() to check if the configured main file exists.
    pub fn has_main_file(&self) -> bool {
        self.absolute_main_file_path().exists()
    }

    /// Get the path to the _generated directory
    pub fn generated_dir(&self) -> std::path::PathBuf {
        self.root.join("_generated")
    }
}

impl PaperConfig {
    /// paper.toml を読み込む
    pub fn from_file(path: impl AsRef<std::path::Path>) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| crate::error::TypstlabError::ConfigParseError(e.to_string()))?;

        toml::from_str(&content).map_err(|e| crate::error::TypstlabError::PaperConfigInvalid {
            paper_id: "unknown".to_string(),
            reason: e.to_string(),
        })
    }

    /// paper.toml に書き込む
    pub fn to_file(&self, path: impl AsRef<std::path::Path>) -> crate::error::Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::error::TypstlabError::ConfigParseError(e.to_string()))?;

        std::fs::write(path.as_ref(), content).map_err(crate::error::TypstlabError::IoError)?;

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
        assert_eq!(config.layout.theme, "default");
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
theme = "ieee"

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
        assert_eq!(config.layout.theme, "ieee");
        assert_eq!(
            config.refs.as_ref().unwrap().sets,
            vec!["core", "report-2026q1"]
        );
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

    // Phase 2 tests: Paper struct

    #[test]
    fn test_paper_load_from_directory() {
        use typstlab_testkit::temp_dir_in_workspace;

        let temp = temp_dir_in_workspace();
        let paper_dir = temp.path().join("report");
        std::fs::create_dir(&paper_dir).unwrap();

        let toml_content = r#"
[paper]
id = "report"
title = "My Report"
language = "en"
date = "2026-01-05"

[output]
name = "report"
"#;
        std::fs::write(paper_dir.join("paper.toml"), toml_content).unwrap();

        let paper = Paper::load(paper_dir).unwrap();
        assert_eq!(paper.id(), "report");
        assert_eq!(paper.config().paper.title, "My Report");
    }

    #[test]
    fn test_paper_load_validates_id_matches_dir() {
        use typstlab_testkit::temp_dir_in_workspace;

        let temp = temp_dir_in_workspace();
        let paper_dir = temp.path().join("report");
        std::fs::create_dir(&paper_dir).unwrap();

        let toml_content = r#"
[paper]
id = "report"
title = "My Report"
language = "en"
date = "2026-01-05"

[output]
name = "report"
"#;
        std::fs::write(paper_dir.join("paper.toml"), toml_content).unwrap();

        // Should succeed because ID matches directory name
        assert!(Paper::load(paper_dir).is_ok());
    }

    #[test]
    fn test_paper_load_fails_on_id_mismatch() {
        use typstlab_testkit::temp_dir_in_workspace;

        let temp = temp_dir_in_workspace();
        let paper_dir = temp.path().join("thesis");
        std::fs::create_dir(&paper_dir).unwrap();

        let toml_content = r#"
[paper]
id = "report"
title = "My Report"
language = "en"
date = "2026-01-05"

[output]
name = "report"
"#;
        std::fs::write(paper_dir.join("paper.toml"), toml_content).unwrap();

        // Should fail because ID "report" doesn't match directory name "thesis"
        let result = Paper::load(paper_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_has_main_file() {
        use typstlab_testkit::temp_dir_in_workspace;

        let temp = temp_dir_in_workspace();
        let paper_dir = temp.path().join("report");
        std::fs::create_dir(&paper_dir).unwrap();

        let toml_content = r#"
[paper]
id = "report"
title = "My Report"
language = "en"
date = "2026-01-05"

[output]
name = "report"
"#;
        std::fs::write(paper_dir.join("paper.toml"), toml_content).unwrap();

        let paper = Paper::load(paper_dir.clone()).unwrap();
        assert!(!paper.has_main_file());

        // Create main.typ
        std::fs::write(paper_dir.join("main.typ"), "// main").unwrap();
        let paper = Paper::load(paper_dir).unwrap();
        assert!(paper.has_main_file());
    }

    #[test]
    fn test_paper_generated_dir_path() {
        use typstlab_testkit::temp_dir_in_workspace;

        let temp = temp_dir_in_workspace();
        let paper_dir = temp.path().join("report");
        std::fs::create_dir(&paper_dir).unwrap();

        let toml_content = r#"
[paper]
id = "report"
title = "My Report"
language = "en"
date = "2026-01-05"

[output]
name = "report"
"#;
        std::fs::write(paper_dir.join("paper.toml"), toml_content).unwrap();

        let paper = Paper::load(paper_dir.clone()).unwrap();
        let generated = paper.generated_dir();
        assert_eq!(generated, paper_dir.join("_generated"));
    }

    #[test]
    fn test_paper_config_with_default_main_file() {
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
        assert_eq!(config.build.main_file, "main.typ");
    }

    #[test]
    fn test_paper_config_with_custom_main_file() {
        let toml = r#"
[paper]
id = "report"
title = "My Report"
language = "en"
date = "2026-01-05"

[build]
main_file = "src/index.typ"

[output]
name = "report"
"#;
        let config: PaperConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.build.main_file, "src/index.typ");
    }

    #[test]
    fn test_paper_config_with_root() {
        let toml = r#"
[paper]
id = "report"
title = "My Report"
language = "en"
date = "2026-01-05"

[build]
main_file = "src/index.typ"
root = "src"

[output]
name = "report"
"#;
        let config: PaperConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.build.main_file, "src/index.typ");
        assert_eq!(config.build.root, Some("src".to_string()));
    }

    #[test]
    fn test_paper_main_file_path_calculation() {
        use typstlab_testkit::temp_dir_in_workspace;

        let temp = temp_dir_in_workspace();
        let paper_dir = temp.path().join("report");
        std::fs::create_dir_all(&paper_dir).unwrap();

        // Test 1: Default main_file (no root)
        let toml_content = r#"
[paper]
id = "report"
title = "My Report"
language = "en"
date = "2026-01-05"

[output]
name = "report"
"#;
        std::fs::write(paper_dir.join("paper.toml"), toml_content).unwrap();

        let paper = Paper::load(paper_dir.clone()).unwrap();
        assert_eq!(paper.main_file_path(), std::path::PathBuf::from("main.typ"));
        assert_eq!(paper.absolute_main_file_path(), paper_dir.join("main.typ"));

        // Test 2: Custom main_file with root
        let toml_content = r#"
[paper]
id = "report"
title = "My Report"
language = "en"
date = "2026-01-05"

[build]
main_file = "index.typ"
root = "src"

[output]
name = "report"
"#;
        std::fs::write(paper_dir.join("paper.toml"), toml_content).unwrap();

        let paper = Paper::load(paper_dir.clone()).unwrap();
        assert_eq!(
            paper.main_file_path(),
            std::path::PathBuf::from("src").join("index.typ")
        );
        assert_eq!(
            paper.absolute_main_file_path(),
            paper_dir.join("src").join("index.typ")
        );
    }

    #[test]
    fn test_paper_root_path_calculation() {
        use typstlab_testkit::temp_dir_in_workspace;

        let temp = temp_dir_in_workspace();
        let paper_dir = temp.path().join("report");
        std::fs::create_dir_all(&paper_dir).unwrap();

        // Test 1: No root specified
        let toml_content = r#"
[paper]
id = "report"
title = "My Report"
language = "en"
date = "2026-01-05"

[output]
name = "report"
"#;
        std::fs::write(paper_dir.join("paper.toml"), toml_content).unwrap();

        let paper = Paper::load(paper_dir.clone()).unwrap();
        assert_eq!(paper.typst_root_dir(), None);

        // Test 2: root specified
        let toml_content = r#"
[paper]
id = "report"
title = "My Report"
language = "en"
date = "2026-01-05"

[build]
root = "src"

[output]
name = "report"
"#;
        std::fs::write(paper_dir.join("paper.toml"), toml_content).unwrap();

        let paper = Paper::load(paper_dir.clone()).unwrap();
        assert_eq!(paper.typst_root_dir(), Some(paper_dir.join("src")));
    }
}
