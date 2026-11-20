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

use crate::ToolChoice;

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
    #[error("invalid requested semantic version '{version}' for {tool}")]
    InvalidRequestedVersion { tool: &'static str, version: String },
    #[error("no compatible {tool} versions for base version '{base_version}'")]
    NoCompatibleToolVersions {
        tool: &'static str,
        base_version: String,
    },
    #[error("{tool} version '{version}' is not listed in the resolver")]
    ToolVersionNotFound { tool: &'static str, version: String },
    #[error("{tool} version '{version}' is incompatible with base version '{base_version}'")]
    IncompatibleToolVersion {
        tool: &'static str,
        base_version: String,
        version: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityTable {
    tool: &'static str,
    /// base version をキーに、その時点で互換とみなせる tool version を保持する。
    versions_by_base: BTreeMap<String, Vec<Version>>,
    all_versions: BTreeSet<Version>,
}

impl CompatibilityTable {
    /// Parses bare SemVer keys and values, skipping resolver metadata.
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
            if matches!(
                key.as_str(),
                "$schema" | "base_url" | "version_pattern" | "ignores"
            ) {
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
                let version = validate_version(tool, version)?;
                all_versions.insert(version.clone());
                compatible_versions.push(version);
            }

            versions_by_base.insert(key.to_string(), compatible_versions);
        }

        Ok(Self {
            tool,
            versions_by_base,
            all_versions,
        })
    }

    pub fn has_base_version(&self, version: &str) -> bool {
        self.versions_by_base
            .contains_key(without_v_prefix(version))
    }

    pub fn is_empty(&self) -> bool {
        self.all_versions.is_empty()
    }

    /// Uses SemVer order, with build metadata as a tie-breaker.
    pub fn latest_version(&self) -> Option<&Version> {
        self.all_versions.last()
    }

    pub(crate) fn compatible_versions(&self, base_version: &str) -> &[Version] {
        self.versions_by_base
            .get(base_version)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Accepts bare SemVer and a single lowercase `v` prefix in user input.
    pub fn resolve_choice(
        &self,
        base_version: &str,
        choice: &ToolChoice,
    ) -> Result<Option<Version>, VersionResolveError> {
        let base_version = without_v_prefix(base_version);
        let compatible = self.compatible_versions(base_version);

        match choice {
            ToolChoice::None => Ok(None),
            ToolChoice::Auto => compatible.iter().max().cloned().map(Some).ok_or_else(|| {
                VersionResolveError::NoCompatibleToolVersions {
                    tool: self.tool,
                    base_version: base_version.to_owned(),
                }
            }),
            ToolChoice::Version(version) => {
                let version = parse_requested_version(self.tool, version)?;
                if compatible.contains(&version) {
                    Ok(Some(version))
                } else if self.all_versions.contains(&version) {
                    Err(VersionResolveError::IncompatibleToolVersion {
                        tool: self.tool,
                        base_version: base_version.to_owned(),
                        version: version.to_string(),
                    })
                } else {
                    Err(VersionResolveError::ToolVersionNotFound {
                        tool: self.tool,
                        version: version.to_string(),
                    })
                }
            }
        }
    }
}

fn without_v_prefix(version: &str) -> &str {
    version.strip_prefix('v').unwrap_or(version)
}

pub(crate) fn parse_requested_version(
    tool: &'static str,
    version: &str,
) -> Result<Version, VersionResolveError> {
    Version::parse(without_v_prefix(version)).map_err(|_| {
        VersionResolveError::InvalidRequestedVersion {
            tool,
            version: version.to_owned(),
        }
    })
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
        assert!(table.has_base_version("v0.1.0"));
        assert!(!table.has_base_version("0.0.1"));
        assert!(!table.is_empty());
    }

    #[test]
    fn rejects_prefixed_version_strings() {
        for json in [r#"{"v0.1.0": ["0.1.0"]}"#, r#"{"0.1.0": ["v0.1.0"]}"#] {
            assert_eq!(
                CompatibilityTable::from_json_str("tool", json),
                Err(VersionResolveError::InvalidVersion {
                    tool: "tool",
                    version: "v0.1.0".to_owned(),
                })
            );
        }
    }

    #[test]
    fn chooses_latest_by_semver_and_only_from_compatible_row() {
        let table = CompatibilityTable::from_json_str(
            "tool",
            r#"{
                "1.0.0": ["0.10.0-rc.2", "0.9.9", "0.10.0-rc.10"],
                "2.0.0": ["0.10.0"]
            }"#,
        )
        .unwrap();
        assert_eq!(table.latest_version(), Some(&Version::new(0, 10, 0)));
        assert_eq!(
            table.resolve_choice("v1.0.0", &ToolChoice::Auto),
            Ok(Some(Version::parse("0.10.0-rc.10").unwrap()))
        );
    }

    #[test]
    fn handles_empty_and_missing_base_rows() {
        let table = CompatibilityTable::from_json_str("tool", r#"{"1.0.0": []}"#).unwrap();
        assert!(table.is_empty());
        assert_eq!(table.latest_version(), None);
        assert!(table.has_base_version("1.0.0"));
        for base in ["1.0.0", "2.0.0"] {
            assert_eq!(
                table.resolve_choice(base, &ToolChoice::Auto),
                Err(VersionResolveError::NoCompatibleToolVersions {
                    tool: "tool",
                    base_version: base.to_owned(),
                })
            );
            assert_eq!(table.resolve_choice(base, &ToolChoice::None), Ok(None));
        }
    }

    #[test]
    fn distinguishes_invalid_unknown_and_incompatible_explicit_versions() {
        let table = CompatibilityTable::from_json_str(
            "tool",
            r#"{"1.0.0": ["1.1.0"], "2.0.0": ["2.1.0"]}"#,
        )
        .unwrap();
        let resolve =
            |version: &str| table.resolve_choice("1.0.0", &ToolChoice::Version(version.to_owned()));

        for version in ["1.1.0", "v1.1.0"] {
            assert_eq!(resolve(version), Ok(Some(Version::new(1, 1, 0))));
        }
        for version in ["v1.1", "vv1.1.0", "V1.1.0"] {
            assert_eq!(
                resolve(version),
                Err(VersionResolveError::InvalidRequestedVersion {
                    tool: "tool",
                    version: version.to_owned(),
                })
            );
        }
        assert_eq!(
            resolve("v3.1.0"),
            Err(VersionResolveError::ToolVersionNotFound {
                tool: "tool",
                version: "3.1.0".to_owned(),
            })
        );
        assert_eq!(
            resolve("2.1.0"),
            Err(VersionResolveError::IncompatibleToolVersion {
                tool: "tool",
                base_version: "1.0.0".to_owned(),
                version: "2.1.0".to_owned(),
            })
        );
    }
}
