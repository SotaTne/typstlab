use semver::Version;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionResolution {
    ResolverJson(&'static str),
    TypstlabVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum VersionResolveError {
    #[error("invalid embedded resolver JSON for {tool}: {message}")]
    InvalidEmbeddedJson { tool: &'static str, message: String },
    #[error("invalid semantic version '{version}' in {tool} resolver JSON")]
    InvalidVersion { tool: &'static str, version: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityTable {
    versions_by_base: BTreeMap<String, Vec<String>>,
    all_versions: BTreeSet<String>,
}

impl CompatibilityTable {
    pub fn from_json_str(tool: &'static str, json: &str) -> Result<Self, VersionResolveError> {
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

        let mut versions_by_base = BTreeMap::new();
        let mut all_versions = BTreeSet::new();

        for (key, value) in object {
            if !is_version_key(key) {
                continue;
            }

            validate_version(tool, key)?;

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
                validate_version(tool, version)?;

                all_versions.insert(version.to_string());
                compatible_versions.push(version.to_string());
            }

            versions_by_base.insert(key.to_string(), compatible_versions);
        }

        Ok(Self {
            versions_by_base,
            all_versions,
        })
    }

    pub fn has_base_version(&self, version: &str) -> bool {
        self.versions_by_base.contains_key(version)
    }

    pub fn is_empty(&self) -> bool {
        self.all_versions.is_empty()
    }
}

fn is_version_key(key: &str) -> bool {
    Version::parse(key).is_ok()
}

fn validate_version(tool: &'static str, version: &str) -> Result<Version, VersionResolveError> {
    Version::parse(version).map_err(|_| VersionResolveError::InvalidVersion {
        tool,
        version: version.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_resolver_json() {
        let table = CompatibilityTable::from_json_str(
            "tool",
            r#"{
                "0.1.0": ["0.1.0"],
                "ignores": ["0.0.1"]
            }"#,
        )
        .unwrap();

        assert!(table.has_base_version("0.1.0"));
        assert!(!table.is_empty());
    }

    #[test]
    fn rejects_prefixed_version_strings() {
        let error = CompatibilityTable::from_json_str(
            "tool",
            r#"{
                "0.1.0": ["0.1.0"]
            }"#,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            VersionResolveError::InvalidVersion { version, .. } if version == "0.1.0"
        ));
    }
}
