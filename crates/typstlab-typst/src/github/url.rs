//! URL construction helpers for GitHub resources

use thiserror::Error;
use url::Url;

/// Builds GitHub base URL
///
/// # Returns
///
/// Base URL for github.com
pub fn github_base_url() -> Result<Url, url::ParseError> {
    Url::parse("https://github.com")
}

/// Builds GitHub API base URL
///
/// # Returns
///
/// Base URL for api.github.com
pub fn github_api_base_url() -> Result<Url, url::ParseError> {
    Url::parse("https://api.github.com")
}

/// Helper to safely add path segments to URL
///
/// # Arguments
///
/// * `url` - URL to modify
/// * `segments` - Path segments to add
///
/// # Errors
///
/// Returns error if URL cannot be a base
pub fn add_path_segments(url: &mut Url, segments: &[&str]) -> Result<(), UrlError> {
    // Clone URL before mutable borrow to avoid borrow checker error
    let url_for_error = url.clone();
    url.path_segments_mut()
        .map_err(|_| UrlError::CannotBeABase { url: url_for_error })?
        .clear()
        .extend(segments);
    Ok(())
}

/// URL construction errors
#[derive(Debug, Error)]
pub enum UrlError {
    /// URL cannot be used as a base
    #[error("URL cannot be a base: {url}")]
    CannotBeABase {
        /// The problematic URL
        url: Url,
    },

    /// Invalid URL parse error
    #[error("Invalid URL: {0}")]
    ParseError(#[from] url::ParseError),
}
