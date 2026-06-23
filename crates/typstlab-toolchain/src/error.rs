use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolchainError {
    #[error("invalid version requirement: {0}")]
    InvalidVersionReq(#[from] semver::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported platform for {tool}: {platform:?}")]
    UnsupportedPlatform {
        tool: &'static str,
        platform: crate::Platform,
    },
    #[error("docs transform failed for {tool}: {message}")]
    DocsTransform { tool: &'static str, message: String },
}
