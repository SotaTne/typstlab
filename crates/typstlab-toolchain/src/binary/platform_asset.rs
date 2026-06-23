use crate::binary::{BinaryArchiveFormat, BinaryLayout};
use crate::{Platform, ToolchainError};

#[derive(Clone, Copy)]
pub struct PlatformAssetSpec {
    /// この asset が対応する target platform。
    pub platform: Platform,
    /// GitHub Release などで使われる、拡張子を除いた asset 名。
    pub asset_name: &'static str,
    /// archive の形式と展開方法。
    pub archive_format: BinaryArchiveFormat,
    /// 展開後の install root から見た executable の相対 path。
    pub executable_relative_path: &'static str,
    /// URL 組み立て用の archive 拡張子。
    pub archive_extension: &'static str,
}

impl PlatformAssetSpec {
    pub fn binary_layout(&self) -> BinaryLayout {
        BinaryLayout {
            archive_format: self.archive_format,
            executable_relative_path: self.executable_relative_path.into(),
        }
    }
}

pub fn resolve_platform_asset(
    tool: &'static str,
    specs: &'static [PlatformAssetSpec],
    platform: Platform,
) -> Result<&'static PlatformAssetSpec, ToolchainError> {
    specs
        .iter()
        .find(|spec| spec.platform == platform)
        .ok_or(ToolchainError::UnsupportedPlatform { tool, platform })
}
