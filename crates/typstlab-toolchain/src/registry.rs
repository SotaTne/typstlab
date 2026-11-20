use crate::binary::{BinaryTool, typst};
use crate::docs::{self, DocsTool};

pub static BINARY_TOOLS: &[&dyn BinaryTool] = &[&typst::TOOL];
pub static DOCS_TOOLS: &[&dyn DocsTool] = &[&docs::typst::TOOL];

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use semver::Version;

    use super::*;
    use crate::binary::VersionParser;
    use crate::{
        Arch, CompatibilityTable, DocsSourceFormat, Os, Platform, ToolchainError, ToolchainTool,
        VersionGuard, VersionResolution,
    };

    fn latest_version(tool: &dyn ToolchainTool) -> Version {
        match tool.version_resolution() {
            VersionResolution::ResolverJson(json) => {
                CompatibilityTable::from_json_str(tool.id(), json)
                    .unwrap()
                    .latest_version()
                    .unwrap_or_else(|| panic!("empty compatibility table for {}", tool.id()))
                    .clone()
            }
            VersionResolution::TypstlabVersion => env!("CARGO_PKG_VERSION").parse().unwrap(),
        }
    }

    #[test]
    fn registered_ids_are_unique_across_kinds() {
        let mut ids = BTreeSet::new();
        let binary_ids = BINARY_TOOLS.iter().map(|tool| tool.id());
        let docs_ids = DOCS_TOOLS.iter().map(|tool| tool.id());
        for id in binary_ids.chain(docs_ids) {
            assert!(!id.is_empty());
            assert!(ids.insert(id), "duplicate toolchain ID: {id}");
        }
    }

    #[test]
    fn registered_binary_commands_preserve_arguments_and_guards() {
        for tool in BINARY_TOOLS {
            let command = tool.version_command().unwrap();
            assert!(!command.invocation.args.is_empty(), "{}", tool.id());
            assert_eq!(command.invocation.version_guard, VersionGuard::Any);
            let VersionParser::Regex { pattern } = command.parser;
            assert!(!pattern.trim().is_empty(), "{}", tool.id());

            let args = vec!["--help".into(), "path with spaces".into()];
            let guard = VersionGuard::any_of(&[">=0.1.0, <0.3.0", ">=0.7.0"]).unwrap();
            let invocation = tool.raw_invocation(args.clone(), guard.clone()).unwrap();
            assert_eq!(invocation.args, args);
            assert_eq!(invocation.version_guard, guard);
        }
    }

    #[test]
    fn registered_binaries_resolve_distributions_for_all_supported_platforms() {
        for &tool in BINARY_TOOLS {
            let version = latest_version(tool).to_string();
            let mut supported = 0;
            for os in [Os::MacOS, Os::Linux, Os::Windows] {
                for arch in [Arch::X86_64, Arch::Aarch64, Arch::Riscv64] {
                    let platform = Platform { os, arch };
                    let distribution = tool.distribution(&version, platform);
                    match tool.asset_name(platform) {
                        Ok(asset) => {
                            supported += 1;
                            assert!(!asset.is_empty());
                            let distribution = distribution.unwrap();
                            assert!(distribution.url.contains(asset), "{}", tool.id());
                            assert_eq!(distribution.layout, tool.binary_layout(platform).unwrap());
                            let path = distribution.layout.executable_relative_path;
                            assert!(path.is_relative() && !path.as_os_str().is_empty());
                        }
                        Err(ToolchainError::UnsupportedPlatform { .. }) => assert!(matches!(
                            distribution,
                            Err(ToolchainError::UnsupportedPlatform { tool: id, platform: rejected })
                                if id == tool.id() && rejected == platform
                        )),
                        Err(error) => panic!("{}: {error}", tool.id()),
                    }
                }
            }
            assert!(supported > 0, "{} has no supported platforms", tool.id());
        }
    }

    #[test]
    fn registered_docs_resolve_sources() {
        for &tool in DOCS_TOOLS {
            let version = latest_version(tool).to_string();
            let source = tool.source(&version).unwrap();
            assert!(!source.url.is_empty(), "{}", tool.id());
            if let DocsSourceFormat::RawFile { file_name } = source.format {
                assert_eq!(Path::new(file_name).file_name().unwrap(), file_name);
            }
        }
    }
}
