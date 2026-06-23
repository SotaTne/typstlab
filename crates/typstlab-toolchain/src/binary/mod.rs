use crate::{
    InstallLayout, InstallSource, Platform, RawCommandFactory, ToolInvocation, ToolchainError,
    VersionResolution,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionParser {
    Regex { pattern: &'static str },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionCommand {
    pub invocation: ToolInvocation,
    pub parser: VersionParser,
}

pub trait BinaryTool: Sync {
    fn id(&self) -> &'static str;
    fn version_resolution(&self) -> VersionResolution;
    fn version_command(&self) -> Result<VersionCommand, ToolchainError>;
    fn asset_name(&self, target_platform: Platform) -> Result<&'static str, ToolchainError>;
    fn install_layout(&self, target_platform: Platform) -> Result<InstallLayout, ToolchainError>;

    fn install_source(
        &self,
        version: &str,
        target_platform: Platform,
    ) -> Result<InstallSource, ToolchainError>;
}

pub trait TypedBinaryTool: BinaryTool {
    type Commands: RawCommandFactory + Sync + 'static;

    fn commands(&self) -> &'static Self::Commands;
}

include!(concat!(env!("OUT_DIR"), "/binary_registry.rs"));

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::{CompatibilityTable, VersionResolution};

    #[test]
    fn generated_registry_contains_typst() {
        assert!(TOOLS.iter().any(|tool| tool.id() == "typst"));
    }

    #[test]
    fn every_binary_tool_has_unique_id() {
        let mut ids = BTreeSet::new();

        for tool in TOOLS {
            assert!(
                ids.insert(tool.id()),
                "duplicate binary tool id: {}",
                tool.id()
            );
        }
    }

    #[test]
    fn every_binary_tool_has_valid_version_resolution() {
        for tool in TOOLS {
            match tool.version_resolution() {
                VersionResolution::ResolverJson(json) => {
                    CompatibilityTable::from_json_str(tool.id(), json).unwrap();
                }
                VersionResolution::TypstlabVersion => {}
            }
        }
    }

    #[test]
    fn every_binary_tool_has_version_command() {
        for tool in TOOLS {
            let command = tool.version_command().unwrap();
            assert!(!command.invocation.executable.is_empty());
            assert!(!command.invocation.args.is_empty());
            assert!(
                command
                    .invocation
                    .version_guard
                    .matches(&semver::Version::new(0, 0, 0))
            );
        }
    }

    #[test]
    fn every_version_command_has_usable_version_guard() {
        for tool in TOOLS {
            let command = tool.version_command().unwrap();
            assert!(
                command
                    .invocation
                    .version_guard
                    .matches(&semver::Version::new(999, 0, 0))
            );
        }
    }

    #[test]
    fn every_binary_tool_has_raw_command_factory() {
        assert_command_contracts();
    }

    #[test]
    fn every_binary_tool_resolves_current_platform_install_source() {
        let platform = Platform::current();

        for tool in TOOLS {
            let source = tool.install_source("0.1.0", platform).unwrap();
            assert!(!source.url.is_empty());
            assert!(
                !source
                    .layout
                    .executable_relative_path
                    .as_os_str()
                    .is_empty()
            );
        }
    }
}
