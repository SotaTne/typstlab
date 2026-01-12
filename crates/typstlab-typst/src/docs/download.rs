//! Typst documentation archive download

use crate::github;
use thiserror::Error;
use url::Url;

/// Maximum documentation archive size (50 MB)
pub const MAX_DOCS_SIZE: u64 = 50 * 1024 * 1024;

/// Builds GitHub archive URL for Typst documentation
///
/// # Arguments
///
/// * `version` - Typst version (e.g., "0.12.0")
///
/// # Returns
///
/// URL for downloading docs archive (tar.gz)
///
/// # Example
///
/// ```no_run
/// # use typstlab_typst::docs::download::build_docs_archive_url;
/// let url = build_docs_archive_url("0.12.0").unwrap();
/// assert_eq!(url.as_str(), "https://github.com/typst/typst/archive/refs/tags/v0.12.0.tar.gz");
/// ```
pub fn build_docs_archive_url(version: &str) -> Result<Url, DocsError> {
    let mut url = github::github_base_url()?;

    let path = format!("v{}.tar.gz", version);
    let segments = &["typst", "typst", "archive", "refs", "tags", &path];

    github::add_path_segments(&mut url, segments)?;

    Ok(url)
}

/// Downloads Typst documentation archive
///
/// # Arguments
///
/// * `version` - Typst version (e.g., "0.12.0")
/// * `verbose` - Enable verbose output
///
/// # Returns
///
/// Downloaded archive bytes
///
/// # Errors
///
/// Returns error if:
/// - URL construction fails
/// - HTTP request fails
/// - Download size exceeds MAX_DOCS_SIZE
pub fn download_docs_archive(version: &str, verbose: bool) -> Result<Vec<u8>, DocsError> {
    let url = build_docs_archive_url(version)?;

    if verbose {
        eprintln!("Downloading from {}...", url);
    }

    // Build client
    let client = github::build_default_client()?;

    // Download with size limit
    let options = github::DownloadOptions {
        expected_size: None, // GitHub doesn't provide content-length for archives
        progress: if verbose {
            Some(|downloaded, _total| {
                eprintln!("Downloaded {} bytes", downloaded);
            })
        } else {
            None
        },
        timeout: None, // Use default
    };

    let bytes = github::download_to_memory(&client, &url, options)?;

    // Verify size limit
    if bytes.len() as u64 > MAX_DOCS_SIZE {
        return Err(DocsError::SizeExceeded {
            size: bytes.len() as u64,
            max: MAX_DOCS_SIZE,
        });
    }

    if verbose {
        eprintln!("Downloaded {} bytes", bytes.len());
    }

    Ok(bytes)
}

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

    /// No documentation files found in archive
    #[error("No documentation files found in archive")]
    NoDocsFound,

    /// Path traversal attempt detected
    #[error("Path traversal detected in archive entry")]
    PathTraversal,
}
