use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolchainError {
    #[error("invalid version requirement: {0}")]
    InvalidVersionReq(#[from] semver::Error),
    #[error("unsupported platform for {tool}: {platform:?}")]
    UnsupportedPlatform {
        tool: &'static str,
        platform: crate::Platform,
    },
}
