use std::collections::BTreeMap;

use semver::Version;
use thiserror::Error;

use crate::toolchain_spec::TYPST_ID;
use crate::version_resolution::parse_requested_version;
use crate::{
    BINARY_TOOLS, CompatibilityTable, DOCS_TOOLS, ToolChoice, ToolchainSpec, ToolchainSpecError,
    ToolchainTool, VersionResolution, VersionResolveError,
};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ToolchainResolveError {
    #[error(transparent)]
    InvalidSpec(#[from] ToolchainSpecError),
    #[error("unknown tool '{id}' in [{section}]")]
    UnknownTool { section: &'static str, id: String },
    #[error("Typst version '{version}' is not listed in the Typst resolver")]
    TypstVersionNotFound { version: String },
    #[error(transparent)]
    Version(#[from] VersionResolveError),
}

/// The explicitly configured, enabled tools and docs with their resolved versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedToolchainSpec {
    pub tools: BTreeMap<&'static str, Version>,
    pub docs: BTreeMap<&'static str, Version>,
}

/// Resolves against the appropriate registry for each section.
/// Entries set to `none` are ignored, including IDs that are no longer registered.
pub fn resolve_toolchain(
    spec: &ToolchainSpec,
) -> Result<ResolvedToolchainSpec, ToolchainResolveError> {
    let typst_version = parse_requested_version(TYPST_ID, spec.typst_version()?)?;
    let base_version = typst_version.to_string();
    let typst = BINARY_TOOLS
        .iter()
        .find(|tool| tool.id() == TYPST_ID)
        .ok_or_else(|| ToolchainResolveError::UnknownTool {
            section: "tools",
            id: TYPST_ID.to_owned(),
        })?;

    let typst_available = match typst.version_resolution() {
        VersionResolution::ResolverJson(json) => {
            let table = CompatibilityTable::from_json_str(typst.id(), json)?;
            table
                .compatible_versions(&base_version)
                .contains(&typst_version)
        }
        VersionResolution::TypstlabVersion => typst_version == typstlab_version(typst.id())?,
    };
    if !typst_available {
        return Err(ToolchainResolveError::TypstVersionNotFound {
            version: base_version,
        });
    }

    let mut tools = resolve_choices(
        spec.tools.iter().filter(|(id, _)| id.as_str() != TYPST_ID),
        BINARY_TOOLS,
        "tools",
        &base_version,
    )?;
    tools.insert(typst.id(), typst_version);
    let docs = resolve_choices(spec.docs.iter(), DOCS_TOOLS, "docs", &base_version)?;
    Ok(ResolvedToolchainSpec { tools, docs })
}

fn resolve_choices<'a, T: ToolchainTool + ?Sized>(
    choices: impl Iterator<Item = (&'a String, &'a ToolChoice)>,
    registry: &[&T],
    section: &'static str,
    base_version: &str,
) -> Result<BTreeMap<&'static str, Version>, ToolchainResolveError> {
    let mut resolved = BTreeMap::new();
    for (id, choice) in choices {
        if *choice == ToolChoice::None {
            continue;
        }
        let tool = registry
            .iter()
            .find(|tool| tool.id() == id)
            .ok_or_else(|| ToolchainResolveError::UnknownTool {
                section,
                id: id.clone(),
            })?;
        if let Some(version) =
            resolve_choice(tool.id(), tool.version_resolution(), base_version, choice)?
        {
            resolved.insert(tool.id(), version);
        }
    }
    Ok(resolved)
}

fn resolve_choice(
    tool: &'static str,
    resolution: VersionResolution,
    base_version: &str,
    choice: &ToolChoice,
) -> Result<Option<Version>, VersionResolveError> {
    match resolution {
        VersionResolution::ResolverJson(json) => {
            CompatibilityTable::from_json_str(tool, json)?.resolve_choice(base_version, choice)
        }
        VersionResolution::TypstlabVersion => {
            let current = typstlab_version(tool)?;
            match choice {
                ToolChoice::None => Ok(None),
                ToolChoice::Auto => Ok(Some(current)),
                ToolChoice::Version(version) => {
                    let version = parse_requested_version(tool, version)?;
                    if version == current {
                        Ok(Some(version))
                    } else {
                        Err(VersionResolveError::ToolVersionNotFound {
                            tool,
                            version: version.to_string(),
                        })
                    }
                }
            }
        }
    }
}

fn typstlab_version(tool: &'static str) -> Result<Version, VersionResolveError> {
    parse_requested_version(tool, env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(toml: &str) -> Result<ResolvedToolchainSpec, ToolchainResolveError> {
        resolve_toolchain(&toml::from_str(toml).unwrap())
    }

    #[test]
    fn resolves_registered_choices_and_omits_disabled_entries() {
        for docs_choice in ["auto", "v0.14.2"] {
            let resolved = resolve(&format!(
                r#"
                    [tools]
                    typst = "v0.14.2"
                    typstyle = "none"
                    [docs]
                    typst-docs = "{docs_choice}"
                    former-docs = "none"
                "#
            ))
            .unwrap();
            assert_eq!(
                resolved.tools,
                BTreeMap::from([("typst", Version::new(0, 14, 2))])
            );
            assert_eq!(
                resolved.docs,
                BTreeMap::from([("typst-docs", Version::new(0, 14, 2))])
            );
        }
        let resolved = resolve("[tools]\ntypst = '0.14.2'\n[docs]\ntypst-docs = 'none'").unwrap();
        assert!(resolved.docs.is_empty());
        assert!(
            resolve("[tools]\ntypst = '0.14.2'")
                .unwrap()
                .docs
                .is_empty()
        );
    }

    #[test]
    fn requires_explicit_typst_in_tools() {
        for spec in [
            "",
            "[docs]\ntypst = '0.14.2'",
            "[tools]\ntypst = 'auto'",
            "[tools]\ntypst = 'none'",
        ] {
            assert_eq!(
                resolve(spec),
                Err(ToolchainResolveError::InvalidSpec(
                    ToolchainSpecError::TypstVersionRequired
                ))
            );
        }
    }

    #[test]
    fn distinguishes_unknown_and_malformed_typst_versions() {
        assert_eq!(
            resolve("[tools]\ntypst = 'v0.11.0'"),
            Err(ToolchainResolveError::TypstVersionNotFound {
                version: "0.11.0".to_owned()
            })
        );
        assert_eq!(
            resolve("[tools]\ntypst = 'v0.14'"),
            Err(ToolchainResolveError::Version(
                VersionResolveError::InvalidRequestedVersion {
                    tool: "typst",
                    version: "v0.14".to_owned()
                }
            ))
        );
    }

    #[test]
    fn rejects_enabled_unknown_and_wrong_section_ids() {
        for choice in ["auto", "0.14.2"] {
            for (config, section, id) in [
                ("typstyle", "tools", "typstyle"),
                ("typst-docs", "tools", "typst-docs"),
                ("[docs]\nmissing", "docs", "missing"),
                ("[docs]\ntypst", "docs", "typst"),
            ] {
                assert_eq!(
                    resolve(&format!("[tools]\ntypst = '0.14.2'\n{config} = '{choice}'")),
                    Err(ToolchainResolveError::UnknownTool {
                        section,
                        id: id.to_owned()
                    })
                );
            }
        }
    }

    #[test]
    fn rejects_incompatible_docs_for_the_selected_typst_version() {
        assert_eq!(
            resolve("[tools]\ntypst = '0.13.0'\n[docs]\ntypst-docs = '0.14.2'"),
            Err(ToolchainResolveError::Version(
                VersionResolveError::IncompatibleToolVersion {
                    tool: "typst-docs",
                    base_version: "0.13.0".to_owned(),
                    version: "0.14.2".to_owned(),
                }
            ))
        );
    }

    #[test]
    fn typstlab_version_resolution_is_independent_of_the_base_version() {
        let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        let resolve =
            |choice| resolve_choice("docs", VersionResolution::TypstlabVersion, "1.0.0", &choice);
        assert_eq!(resolve(ToolChoice::None), Ok(None));
        assert_eq!(resolve(ToolChoice::Auto), Ok(Some(current.clone())));
        assert_eq!(
            resolve(ToolChoice::Version(format!("v{current}"))),
            Ok(Some(current.clone()))
        );
        let other = Version::new(current.major, current.minor, current.patch + 1);
        assert_eq!(
            resolve(ToolChoice::Version(other.to_string())),
            Err(VersionResolveError::ToolVersionNotFound {
                tool: "docs",
                version: other.to_string(),
            })
        );
    }
}
