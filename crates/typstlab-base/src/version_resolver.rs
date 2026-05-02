use semver::Version as SemverVersion;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;
use thiserror::Error;

const TYPST_JSON: &str = include_str!("version_resolver_jsons/typst.json");
const TYPST_DOCS_JSON: &str = include_str!("version_resolver_jsons/type_docs.json");
const TYPSTYLE_JSON: &str = include_str!("version_resolver_jsons/typstyle.json");

static TYPST_TABLE: OnceLock<Result<CompatibilityTable, VersionResolveError>> = OnceLock::new();
static TYPST_DOCS_TABLE: OnceLock<Result<CompatibilityTable, VersionResolveError>> =
    OnceLock::new();
static TYPSTYLE_TABLE: OnceLock<Result<CompatibilityTable, VersionResolveError>> = OnceLock::new();

static TYPST_RESOLVER: JsonToolResolver = JsonToolResolver {
    name: "typst",
    json: TYPST_JSON,
    cache: &TYPST_TABLE,
};
static TYPST_DOCS_RESOLVER: JsonToolResolver = JsonToolResolver {
    name: "typst_docs",
    json: TYPST_DOCS_JSON,
    cache: &TYPST_DOCS_TABLE,
};
static TYPSTYLE_RESOLVER: JsonToolResolver = JsonToolResolver {
    name: "typstyle",
    json: TYPSTYLE_JSON,
    cache: &TYPSTYLE_TABLE,
};

pub fn get_latest_typst() -> &'static str {
    "0.14.2"
}

fn default_typst_version() -> String {
    get_latest_typst().to_string()
}

fn default_typst_docs_choice() -> ToolChoice {
    ToolChoice::Auto
}

fn default_typstyle_choice() -> ToolChoice {
    ToolChoice::None
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    None,
    Version(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectToolChain {
    #[serde(default = "default_typst_version")]
    pub typst: String,
    #[serde(default = "default_typst_docs_choice")]
    pub typst_docs: ToolChoice,
    #[serde(default = "default_typstyle_choice")]
    pub typstyle: ToolChoice,
}

impl Default for ProjectToolChain {
    fn default() -> Self {
        Self {
            typst: default_typst_version(),
            typst_docs: default_typst_docs_choice(),
            typstyle: default_typstyle_choice(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedToolChain {
    pub typst: String,
    pub typst_docs: Option<String>,
    pub typstyle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum VersionResolveError {
    #[error("invalid embedded resolver JSON for {tool}: {message}")]
    InvalidEmbeddedJson { tool: &'static str, message: String },

    #[error("typst version '{version}' is not listed in typst resolver JSON")]
    TypstVersionNotFound { version: String },

    #[error("no compatible {tool} versions are listed for typst {typst_version}")]
    NoCompatibleToolVersions {
        tool: &'static str,
        typst_version: String,
    },

    #[error("{tool} version '{version}' is not listed in resolver JSON")]
    ToolVersionNotFound { tool: &'static str, version: String },

    #[error("{tool} version '{version}' is not compatible with typst {typst_version}")]
    IncompatibleToolVersion {
        tool: &'static str,
        typst_version: String,
        version: String,
    },

    #[error("invalid semantic version '{version}' in {tool} resolver JSON")]
    InvalidVersion { tool: &'static str, version: String },
}

pub fn resolve_toolchain(
    toolchain: &ProjectToolChain,
) -> Result<ResolvedToolChain, VersionResolveError> {
    let typst = normalize_version(&toolchain.typst);
    TYPST_RESOLVER.ensure_typst_version_exists(&typst)?;

    Ok(ResolvedToolChain {
        typst: typst.clone(),
        typst_docs: TYPST_DOCS_RESOLVER.resolve_choice(&typst, &toolchain.typst_docs)?,
        typstyle: TYPSTYLE_RESOLVER.resolve_choice(&typst, &toolchain.typstyle)?,
    })
}

trait VersionResolver {
    fn tool_name(&self) -> &'static str;
    fn table(&self) -> Result<&CompatibilityTable, VersionResolveError>;

    fn resolve_choice(
        &self,
        typst_version: &str,
        choice: &ToolChoice,
    ) -> Result<Option<String>, VersionResolveError> {
        match choice {
            ToolChoice::None => Ok(None),
            ToolChoice::Auto => self.latest_compatible(typst_version).map(Some),
            ToolChoice::Version(version) => {
                let version = normalize_version(version);
                self.ensure_compatible(typst_version, &version)?;
                Ok(Some(version))
            }
        }
    }

    fn latest_compatible(&self, typst_version: &str) -> Result<String, VersionResolveError> {
        self.table()?
            .latest_compatible(self.tool_name(), typst_version)?
            .ok_or_else(|| VersionResolveError::NoCompatibleToolVersions {
                tool: self.tool_name(),
                typst_version: typst_version.to_string(),
            })
    }

    fn ensure_compatible(
        &self,
        typst_version: &str,
        version: &str,
    ) -> Result<(), VersionResolveError> {
        self.table()?
            .ensure_compatible(self.tool_name(), typst_version, version)
    }
}

#[derive(Debug)]
struct JsonToolResolver {
    name: &'static str,
    json: &'static str,
    cache: &'static OnceLock<Result<CompatibilityTable, VersionResolveError>>,
}

impl JsonToolResolver {
    fn ensure_typst_version_exists(&self, version: &str) -> Result<(), VersionResolveError> {
        if self.table()?.has_typst_version(version) {
            Ok(())
        } else {
            Err(VersionResolveError::TypstVersionNotFound {
                version: version.to_string(),
            })
        }
    }
}

impl VersionResolver for JsonToolResolver {
    fn tool_name(&self) -> &'static str {
        self.name
    }

    fn table(&self) -> Result<&CompatibilityTable, VersionResolveError> {
        self.cache
            .get_or_init(|| CompatibilityTable::from_json_str(self.name, self.json))
            .as_ref()
            .map_err(Clone::clone)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompatibilityTable {
    versions_by_typst: BTreeMap<String, Vec<String>>,
    all_versions: BTreeSet<String>,
}

impl CompatibilityTable {
    fn from_json_str(tool: &'static str, json: &str) -> Result<Self, VersionResolveError> {
        let value: Value = serde_json::from_str(json).map_err(|error| {
            VersionResolveError::InvalidEmbeddedJson {
                tool,
                message: error.to_string(),
            }
        })?;
        let object = value
            .as_object()
            .ok_or_else(|| VersionResolveError::InvalidEmbeddedJson {
                tool,
                message: "expected top-level object".to_string(),
            })?;

        let mut versions_by_typst = BTreeMap::new();
        let mut all_versions = BTreeSet::new();

        for (key, value) in object {
            if !is_version_key(key) {
                continue;
            }

            let typst_version = normalize_version(key);
            validate_version(tool, &typst_version)?;

            let versions =
                value
                    .as_array()
                    .ok_or_else(|| VersionResolveError::InvalidEmbeddedJson {
                        tool,
                        message: format!("expected array for version key '{key}'"),
                    })?;
            let mut compatible_versions = Vec::with_capacity(versions.len());

            for version in versions {
                let version =
                    version
                        .as_str()
                        .ok_or_else(|| VersionResolveError::InvalidEmbeddedJson {
                            tool,
                            message: format!("expected string version under key '{key}'"),
                        })?;
                let version = normalize_version(version);
                validate_version(tool, &version)?;
                all_versions.insert(version.clone());
                compatible_versions.push(version);
            }

            versions_by_typst.insert(typst_version, compatible_versions);
        }

        Ok(Self {
            versions_by_typst,
            all_versions,
        })
    }

    fn has_typst_version(&self, typst_version: &str) -> bool {
        self.versions_by_typst.contains_key(typst_version)
    }

    fn compatible_versions(&self, typst_version: &str) -> Option<&[String]> {
        self.versions_by_typst.get(typst_version).map(Vec::as_slice)
    }

    fn latest_compatible(
        &self,
        tool: &'static str,
        typst_version: &str,
    ) -> Result<Option<String>, VersionResolveError> {
        let Some(versions) = self.compatible_versions(typst_version) else {
            return Ok(None);
        };

        let latest = versions
            .iter()
            .map(|version| {
                validate_version(tool, version)?;
                let parsed =
                    SemverVersion::parse(version).expect("version was validated immediately above");
                Ok((parsed, version))
            })
            .collect::<Result<Vec<_>, VersionResolveError>>()?
            .into_iter()
            .max_by(|(left, _), (right, _)| left.cmp(right))
            .map(|(_, version)| version.clone());

        Ok(latest)
    }

    fn ensure_compatible(
        &self,
        tool: &'static str,
        typst_version: &str,
        version: &str,
    ) -> Result<(), VersionResolveError> {
        validate_version(tool, version)?;
        let versions = self.compatible_versions(typst_version).unwrap_or(&[]);

        if versions.iter().any(|candidate| candidate == version) {
            return Ok(());
        }

        if self.all_versions.contains(version) {
            Err(VersionResolveError::IncompatibleToolVersion {
                tool,
                typst_version: typst_version.to_string(),
                version: version.to_string(),
            })
        } else {
            Err(VersionResolveError::ToolVersionNotFound {
                tool,
                version: version.to_string(),
            })
        }
    }
}

fn normalize_version(version: &str) -> String {
    version.strip_prefix('v').unwrap_or(version).to_string()
}

fn is_version_key(key: &str) -> bool {
    SemverVersion::parse(key.strip_prefix('v').unwrap_or(key)).is_ok()
}

fn validate_version(tool: &'static str, version: &str) -> Result<(), VersionResolveError> {
    SemverVersion::parse(version)
        .map(|_| ())
        .map_err(|_| VersionResolveError::InvalidVersion {
            tool,
            version: version.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_latest_typst_is_stable_default() {
        assert_eq!(get_latest_typst(), "0.14.2");
    }

    #[test]
    fn test_default_project_toolchain_uses_latest_typst_and_auto_docs() {
        assert_eq!(
            ProjectToolChain::default(),
            ProjectToolChain {
                typst: "0.14.2".to_string(),
                typst_docs: ToolChoice::Auto,
                typstyle: ToolChoice::None,
            }
        );
    }

    #[test]
    fn test_resolve_toolchain_auto_picks_latest_compatible_versions() {
        let resolved = resolve_toolchain(&ProjectToolChain {
            typst: "0.14.2".to_string(),
            typst_docs: ToolChoice::Auto,
            typstyle: ToolChoice::Auto,
        })
        .unwrap();

        assert_eq!(
            resolved,
            ResolvedToolChain {
                typst: "0.14.2".to_string(),
                typst_docs: Some("0.14.2".to_string()),
                typstyle: Some("0.14.2".to_string()),
            }
        );
    }

    #[test]
    fn test_resolve_toolchain_none_skips_optional_tool() {
        let resolved = resolve_toolchain(&ProjectToolChain {
            typst: "0.14.2".to_string(),
            typst_docs: ToolChoice::None,
            typstyle: ToolChoice::None,
        })
        .unwrap();

        assert_eq!(resolved.typst, "0.14.2");
        assert_eq!(resolved.typst_docs, None);
        assert_eq!(resolved.typstyle, None);
    }

    #[test]
    fn test_resolve_toolchain_accepts_explicit_compatible_version() {
        let resolved = resolve_toolchain(&ProjectToolChain {
            typst: "0.14.2".to_string(),
            typst_docs: ToolChoice::Version("0.14.2".to_string()),
            typstyle: ToolChoice::None,
        })
        .unwrap();

        assert_eq!(resolved.typst_docs.as_deref(), Some("0.14.2"));
    }

    #[test]
    fn test_resolve_toolchain_rejects_unknown_typst_version() {
        let error = resolve_toolchain(&ProjectToolChain {
            typst: "9.9.9".to_string(),
            typst_docs: ToolChoice::None,
            typstyle: ToolChoice::None,
        })
        .unwrap_err();

        assert!(matches!(
            error,
            VersionResolveError::TypstVersionNotFound { version } if version == "9.9.9"
        ));
    }

    #[test]
    fn test_latest_compatible_uses_semver_order() {
        let resolver = resolver_with_json(
            "test_tool",
            r#"{
                "0.14.2": ["0.14.2", "0.14.10", "0.14.9"]
            }"#,
        );

        assert_eq!(
            resolver.latest_compatible("0.14.2").unwrap(),
            "0.14.10".to_string()
        );
    }

    #[test]
    fn test_auto_reports_no_compatible_versions_for_empty_candidate_list() {
        let resolver = resolver_with_json(
            "test_tool",
            r#"{
                "0.14.2": []
            }"#,
        );

        let error = resolver
            .resolve_choice("0.14.2", &ToolChoice::Auto)
            .unwrap_err();

        assert!(matches!(
            error,
            VersionResolveError::NoCompatibleToolVersions {
                tool: "test_tool",
                typst_version
            } if typst_version == "0.14.2"
        ));
    }

    #[test]
    fn test_explicit_version_reports_not_found_before_compatibility() {
        let resolver = resolver_with_json(
            "test_tool",
            r#"{
                "0.14.2": ["1.0.0"],
                "0.14.1": ["1.1.0"]
            }"#,
        );

        let error = resolver.ensure_compatible("0.14.2", "9.9.9").unwrap_err();

        assert!(matches!(
            error,
            VersionResolveError::ToolVersionNotFound {
                tool: "test_tool",
                version
            } if version == "9.9.9"
        ));
    }

    #[test]
    fn test_explicit_version_reports_incompatible_when_version_exists_elsewhere() {
        let resolver = resolver_with_json(
            "test_tool",
            r#"{
                "0.14.2": ["1.0.0"],
                "0.14.1": ["1.1.0"]
            }"#,
        );

        let error = resolver.ensure_compatible("0.14.2", "1.1.0").unwrap_err();

        assert!(matches!(
            error,
            VersionResolveError::IncompatibleToolVersion {
                tool: "test_tool",
                typst_version,
                version
            } if typst_version == "0.14.2" && version == "1.1.0"
        ));
    }

    struct TestResolver {
        name: &'static str,
        table: CompatibilityTable,
    }

    impl VersionResolver for TestResolver {
        fn tool_name(&self) -> &'static str {
            self.name
        }

        fn table(&self) -> Result<&CompatibilityTable, VersionResolveError> {
            Ok(&self.table)
        }
    }

    fn resolver_with_json(name: &'static str, json: &str) -> TestResolver {
        TestResolver {
            name,
            table: CompatibilityTable::from_json_str(name, json).unwrap(),
        }
    }
}
