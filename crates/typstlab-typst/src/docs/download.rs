//! Typst documentation error types and constants

use crate::github;
use thiserror::Error;

/// Maximum documentation download size (50 MB)
pub const MAX_DOCS_SIZE: u64 = 50 * 1024 * 1024;

/// Documentation download errors
#[derive(Debug, Error)]
pub enum DocsError {
    /// Download size exceeds maximum
    #[error("Download size ({size} bytes) exceeds maximum ({max} bytes)")]
    SizeExceeded {
        /// Actual size
        size: u64,
        /// Maximum allowed size
        max: u64,
    },

    /// GitHub download error
    #[error("GitHub error: {0}")]
    GitHubError(#[from] github::DownloadError),

    /// URL construction error
    #[error("URL error: {0}")]
    UrlError(#[from] github::UrlError),

    /// URL parse error
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),

    /// HTTP client error
    #[error("HTTP client error: {0}")]
    HttpClientError(#[from] reqwest::Error),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// File lock acquisition failed
    #[error("Failed to acquire file lock: {0}")]
    LockError(String),

    /// Documentation JSON download error
    #[error("Documentation JSON error: {0}")]
    DocsJsonError(#[from] super::download_json::DocsJsonError),

    /// Schema validation error
    #[error("Schema error: {0}")]
    SchemaError(#[from] super::schema::SchemaError),

    /// Markdown generation error
    #[error("Generation error: {0}")]
    GenerateError(#[from] super::generate::GenerateError),

    /// JSON parse error
    #[error("JSON parse error: {0}")]
    JsonParseError(#[from] serde_json::Error),
}
