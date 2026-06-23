/*
 * これは、 typstlab-toolchain 内での tool version 解決のためのモジュールです。
 * resolver.json を使うか、typstlab 自身の version を使うかを決定し、互換性表を構築します。
 * また、主なパース処理などはresolver.json の構造に依存しており、 tool ごとに異なる resolver.json
 * を扱うことができます。
 */

use semver::Version;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionResolution {
    /// tool ごとの resolver.json を使い、base version から利用可能な version を解決する。
    /// typst のように外部リリースとの互換性表を持つ tool はこの経路を使う。
    ResolverJson(&'static str),
    /// typstlab 自身のバージョンをそのまま tool の version として扱う。
    /// typstlab のドキュメントなど、外部の互換性表を持たない配布物で使う。
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
    /// base version をキーに、その時点で互換とみなせる tool version を保持する。
    versions_by_base: BTreeMap<String, Vec<String>>,
    /// resolver.json 内に登場した全 version。検証や status 表示で使うために保持する。
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
                "0.1.0": ["v0.1.0"]
            }"#,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            VersionResolveError::InvalidVersion { version, .. } if version == "v0.1.0"
        ));
    }
}
