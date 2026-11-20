use crate::VersionResolution;

/// toolchain resolver が binary/docs を共通に扱うための最小 protocol。
///
/// コマンド・配布元の詳細は BinaryTool / DocsTool に残し、ここでは識別と
/// version 解決方法だけを扱う。
pub trait ToolchainTool: Sync {
    fn id(&self) -> &'static str;
    fn version_resolution(&self) -> VersionResolution;
}
