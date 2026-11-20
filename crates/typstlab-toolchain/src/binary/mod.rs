use std::ffi::OsString;

use crate::{Platform, ToolchainError, ToolchainTool};

pub mod command;
pub mod distribution;
pub mod github;
pub mod platform_asset;
pub mod typst;
pub mod version;

pub use command::{RawCommandFactory, ToolCommand, ToolInvocation, VersionGuard};
pub use distribution::{BinaryArchiveFormat, BinaryDistribution, BinaryLayout};
pub use github::{GithubBinaryTool, GithubReleaseSource};
pub use platform_asset::{PlatformAssetSpec, resolve_platform_asset};
pub use version::{VersionCommand, VersionParser, VersionProbe};

/// compile-time に登録された binary tool の共通 protocol。
/// ここには「配布物をどう解決するか」だけを置き、実行場所や Store は持たせない。
pub trait BinaryTool: ToolchainTool {
    /// installed binary から実際の version を読むための command を返す。
    fn version_command(&self) -> Result<VersionCommand, ToolchainError>;

    /// registry 経由で、型付き API にないコマンドの引数とバージョン条件を指定する。
    fn raw_invocation(
        &self,
        args: Vec<OsString>,
        guard: VersionGuard,
    ) -> Result<ToolInvocation, ToolchainError>;

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
