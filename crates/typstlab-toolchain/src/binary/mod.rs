use crate::{Platform, ToolchainError, VersionResolution};

pub mod command;
pub mod distribution;
pub mod github;
pub mod platform_asset;
pub mod version;

pub use command::{RawCommandFactory, ToolCommand, ToolInvocation, VersionGuard};
pub use distribution::{BinaryArchiveFormat, BinaryDistribution, BinaryLayout};
pub use github::{GithubBinaryTool, GithubReleaseSource};
pub use platform_asset::{PlatformAssetSpec, resolve_platform_asset};
pub use version::{VersionCommand, VersionParser, VersionProbe};

/// compile-time に登録された binary tool の共通 protocol。
/// ここには「配布物をどう解決するか」だけを置き、実行場所や Store は持たせない。
pub trait BinaryTool: Sync {
    /// toolchain 全体で一意な tool id。
    fn id(&self) -> &'static str;

    /// version を resolver.json で解決するか、typstlab 自身の version に揃えるかを返す。
    fn version_resolution(&self) -> VersionResolution;

    /// installed binary から実際の version を読むための command を返す。
    fn version_command(&self) -> Result<VersionCommand, ToolchainError>;

    /// target platform に対応する配布 asset 名を返す。
    fn asset_name(&self, target_platform: Platform) -> Result<&'static str, ToolchainError>;

    /// target platform の archive 展開方法と executable の位置を返す。
    fn binary_layout(&self, target_platform: Platform) -> Result<BinaryLayout, ToolchainError>;

    /// version と target platform から、download URL と配置 layout を確定する。
    fn distribution(
        &self,
        version: &str,
        target_platform: Platform,
    ) -> Result<BinaryDistribution, ToolchainError>;
}

/// 型付き command API を持つ binary tool。
/// typst の compile/query のような command を tool ごとの型として公開するための層。
pub trait TypedBinaryTool: BinaryTool {
    type Commands: RawCommandFactory + Sync + 'static;

    fn commands(&self) -> &'static Self::Commands;
}

#[cfg(test)]
/// `RawCommandFactory` を実装しているかを、型制約だけで test するための補助関数。
/// 実行時の意味はなく、生成された registry が contract を満たすかをコンパイル時に固定する。
pub fn assert_raw_command_factory<T: RawCommandFactory>(_factory: &T) {}

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
        // build.rs が生成した registry 全体に対して、COMMAND が RawCommandFactory を満たすことを固定する。
        assert_command_contracts();
    }

    #[test]
    fn every_binary_tool_resolves_current_platform_distribution() {
        let platform = Platform::current();

        for tool in TOOLS {
            let distribution = tool.distribution("0.1.0", platform).unwrap();
            assert!(!distribution.url.is_empty());
            assert!(
                !distribution
                    .layout
                    .executable_relative_path
                    .as_os_str()
                    .is_empty()
            );
        }
    }
}
