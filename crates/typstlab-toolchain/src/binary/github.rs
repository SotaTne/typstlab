use crate::binary::RawCommandFactory;
use crate::binary::{
    BinaryDistribution, BinaryLayout, PlatformAssetSpec, VersionCommand, VersionProbe,
    resolve_platform_asset,
};
use crate::{Platform, ToolchainError, ToolchainTool, VersionResolution};

use super::{BinaryTool, TypedBinaryTool};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GithubReleaseSource {
    /// GitHub owner または organization。
    pub owner: &'static str,
    /// release asset を持つ repository 名。
    pub repo: &'static str,
    /// tag が `v0.1.0` 形式なら `v`、`0.1.0` 形式なら空文字を指定する。
    pub tag_prefix: &'static str,
}

impl GithubReleaseSource {
    pub fn asset_url(&self, version: &str, asset_name: &str, extension: &str) -> String {
        format!(
            "https://github.com/{owner}/{repo}/releases/download/{tag_prefix}{version}/{asset_name}.{extension}",
            owner = self.owner,
            repo = self.repo,
            tag_prefix = self.tag_prefix,
        )
    }
}

pub struct GithubBinaryTool<C>
where
    C: RawCommandFactory + Sync + 'static,
{
    /// toolchain 内で一意な tool id。
    pub id: &'static str,
    /// tool 固有の型付き command factory。
    pub commands: &'static C,
    /// GitHub Release から asset URL を作るための最小情報。
    pub github: GithubReleaseSource,
    /// version を resolver.json で解決するか、typstlab 自身の version に揃えるか。
    pub version_resolution: VersionResolution,
    /// installed binary から version を読むための command と parser。
    pub version_probe: VersionProbe,
    /// platform ごとの asset 名・archive 形式・binary 位置。
    pub platforms: &'static [PlatformAssetSpec],
}

impl<C> ToolchainTool for GithubBinaryTool<C>
where
    C: RawCommandFactory + Sync + 'static,
{
    fn id(&self) -> &'static str {
        self.id
    }

    fn version_resolution(&self) -> VersionResolution {
        self.version_resolution
    }
}

impl<C> BinaryTool for GithubBinaryTool<C>
where
    C: RawCommandFactory + Sync + 'static,
{
    fn version_command(&self) -> Result<VersionCommand, ToolchainError> {
        Ok(self.version_probe.to_command())
    }

    fn asset_name(&self, target_platform: Platform) -> Result<&'static str, ToolchainError> {
        Ok(resolve_platform_asset(self.id, self.platforms, target_platform)?.asset_name)
    }

    fn binary_layout(&self, target_platform: Platform) -> Result<BinaryLayout, ToolchainError> {
        Ok(resolve_platform_asset(self.id, self.platforms, target_platform)?.binary_layout())
    }

    fn distribution(
        &self,
        version: &str,
        target_platform: Platform,
    ) -> Result<BinaryDistribution, ToolchainError> {
        let target = resolve_platform_asset(self.id, self.platforms, target_platform)?;
        Ok(BinaryDistribution {
            url: self
                .github
                .asset_url(version, target.asset_name, target.archive_extension),
            layout: target.binary_layout(),
        })
    }
}

impl<C> TypedBinaryTool for GithubBinaryTool<C>
where
    C: RawCommandFactory + Sync + 'static,
{
    type Commands = C;

    fn commands(&self) -> &'static Self::Commands {
        self.commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_release_source_builds_asset_url() {
        let source = GithubReleaseSource {
            owner: "typst",
            repo: "typst",
            tag_prefix: "v",
        };

        assert_eq!(
            source.asset_url("0.14.2", "typst-aarch64-apple-darwin", "tar.xz"),
            "https://github.com/typst/typst/releases/download/v0.14.2/typst-aarch64-apple-darwin.tar.xz"
        );
    }
}
