//! GitHub Release binary download and installation
//!
//! This module handles downloading pre-built Typst binaries from GitHub Releases.
//!
//! # URL Safety
//!
//! All URLs use `url::Url` type for:
//! - Automatic URL encoding and validation
//! - Prevention of path traversal attacks
//! - Type safety throughout the API
//! - Validation at construction time, not just at HTTP request time
//!
//! Version strings are automatically encoded when constructing GitHub API URLs,
//! preventing injection attacks from malicious version strings like `../../../etc/passwd`.

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

/// Errors that can occur during GitHub Release operations
#[derive(Debug, Error)]
pub enum ReleaseError {
    /// Network request failed
    #[error("Failed to fetch release metadata: {0}")]
    NetworkError(#[from] reqwest::Error),

    /// Release not found (404)
    #[error("Release '{version}' not found")]
    NotFound { version: String },

    /// GitHub API rate limit or permission error (403)
    #[error("GitHub API access forbidden (rate limit or permissions)")]
    Forbidden,

    /// Invalid JSON response
    #[error("Failed to parse GitHub API response: {0}")]
    InvalidJson(#[from] serde_json::Error),

    /// Invalid URL construction
    #[error("Failed to construct GitHub API URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// URL cannot be a base (missing scheme/host)
    #[error("Base URL cannot be used for joining: {url}")]
    InvalidBaseUrl { url: String },

    /// No matching asset found for platform
    #[error("No asset found for {os} {arch} in release '{version}'")]
    AssetNotFound {
        version: String,
        os: String,
        arch: String,
    },

    /// Download failed (network/HTTP error)
    #[error("Failed to download asset from {url}: {source}")]
    DownloadFailed {
        url: Url,
        #[source]
        source: reqwest::Error,
    },

    /// Downloaded file size mismatch
    #[error("Downloaded file size mismatch: expected {expected} bytes, got {actual} bytes")]
    SizeMismatch { expected: u64, actual: u64 },

    /// I/O error during download/extraction
    #[error("I/O error during {operation}: {source}")]
    IoError {
        operation: String,
        #[source]
        source: std::io::Error,
    },

    /// Archive extraction failed
    #[error("Failed to extract {archive_type} archive: {reason}")]
    ExtractionFailed {
        archive_type: String,
        reason: String,
    },

    /// Binary not found in extracted archive
    #[error("Binary '{binary_name}' not found in archive")]
    BinaryNotFoundInArchive { binary_name: String },

    /// Insufficient disk space
    #[error("Insufficient disk space for download (required: {required} bytes)")]
    InsufficientDiskSpace { required: u64 },
}

/// GitHub Release metadata from API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Release {
    /// Release tag name (e.g., "v0.17.0")
    pub tag_name: String,
    /// List of downloadable assets
    pub assets: Vec<Asset>,
}

/// GitHub Release asset (downloadable file)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Asset {
    /// Asset filename (e.g., "typst-x86_64-apple-darwin.tar.gz")
    pub name: String,
    /// Direct download URL
    #[serde(deserialize_with = "deserialize_url")]
    pub browser_download_url: Url,
    /// File size in bytes
    pub size: u64,
}

/// Custom deserializer to convert String to Url with validation
fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
    D: Deserializer<'de>,
{
    let url_str = String::deserialize(deserializer)?;
    Url::parse(&url_str).map_err(de::Error::custom)
}

/// Fetches release metadata from GitHub API
///
/// # Arguments
///
/// * `version` - Version tag (e.g., "v0.17.0") or "latest"
///
/// # Errors
///
/// - `ReleaseError::NotFound` if the release doesn't exist (404)
/// - `ReleaseError::Forbidden` if rate limited or no access (403)
/// - `ReleaseError::NetworkError` for network failures
/// - `ReleaseError::InvalidJson` if response is not valid JSON
///
/// # Examples
///
/// ```no_run
/// use typstlab_typst::install::release::fetch_release_metadata;
///
/// let release = fetch_release_metadata("v0.17.0")?;
/// assert_eq!(release.tag_name, "v0.17.0");
///
/// let latest = fetch_release_metadata("latest")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn fetch_release_metadata(version: &str) -> Result<Release, ReleaseError> {
    fetch_release_metadata_from_url("https://api.github.com", version)
}

/// Internal function for fetching release metadata with configurable base URL
/// (Allows dependency injection for testing)
fn fetch_release_metadata_from_url(base_url: &str, version: &str) -> Result<Release, ReleaseError> {
    let url = build_release_url(base_url, version)?;
    let client = build_http_client()?;
    let response = client.get(url).send()?;
    parse_release_response(response, version)
}

/// Builds the GitHub API URL for the given version
///
/// # Security
///
/// - Validates base URL format
/// - Properly encodes version string in URL path
/// - Prevents path traversal attacks
///
/// # Errors
///
/// - `ReleaseError::InvalidUrl` if base_url is malformed
/// - `ReleaseError::InvalidBaseUrl` if base_url cannot be used for joining
fn build_release_url(base_url: &str, version: &str) -> Result<Url, ReleaseError> {
    // Validate version is not empty
    if version.is_empty() {
        return Err(ReleaseError::InvalidUrl(url::ParseError::EmptyHost));
    }

    // Parse and validate base URL
    let mut url = Url::parse(base_url)?;

    // Build path based on version
    let path_segments = if version == "latest" {
        vec!["repos", "typst", "typst", "releases", "latest"]
    } else {
        vec!["repos", "typst", "typst", "releases", "tags", version]
    };

    // Use path_segments_mut for proper encoding
    url.path_segments_mut()
        .map_err(|_| ReleaseError::InvalidBaseUrl {
            url: base_url.to_string(),
        })?
        .clear()
        .extend(path_segments);

    Ok(url)
}

/// Builds HTTP client with appropriate user agent
fn build_http_client() -> Result<reqwest::blocking::Client, ReleaseError> {
    reqwest::blocking::Client::builder()
        .user_agent("typstlab")
        .build()
        .map_err(ReleaseError::from)
}

/// Parses HTTP response into Release or appropriate error
fn parse_release_response(
    response: reqwest::blocking::Response,
    version: &str,
) -> Result<Release, ReleaseError> {
    match response.status() {
        reqwest::StatusCode::OK => {
            let text = response.text()?;
            let release = serde_json::from_str::<Release>(&text)?;
            Ok(release)
        }
        reqwest::StatusCode::NOT_FOUND => Err(ReleaseError::NotFound {
            version: version.to_string(),
        }),
        reqwest::StatusCode::FORBIDDEN => Err(ReleaseError::Forbidden),
        _ => {
            response.error_for_status()?;
            unreachable!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test fetch_release_metadata with valid version returns release
    #[test]
    fn test_fetch_release_metadata_valid_version_returns_release() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/repos/typst/typst/releases/tags/v0.17.0")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "tag_name": "v0.17.0",
                "assets": [{
                    "name": "typst-x86_64-apple-darwin.tar.gz",
                    "browser_download_url": "https://github.com/typst/typst/releases/download/v0.17.0/typst-x86_64-apple-darwin.tar.gz",
                    "size": 12345678
                }]
            }"#,
            )
            .create();

        let result = fetch_release_metadata_from_url(&server.url(), "v0.17.0");
        assert!(result.is_ok(), "Expected successful fetch");

        let release = result.unwrap();
        assert_eq!(release.tag_name, "v0.17.0");
        assert_eq!(release.assets.len(), 1);
        assert_eq!(release.assets[0].name, "typst-x86_64-apple-darwin.tar.gz");

        mock.assert();
    }

    /// Test fetch_release_metadata with latest returns release
    #[test]
    fn test_fetch_release_metadata_latest_returns_release() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/repos/typst/typst/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "tag_name": "v0.18.0",
                "assets": [{
                    "name": "typst-x86_64-apple-darwin.tar.gz",
                    "browser_download_url": "https://github.com/typst/typst/releases/download/v0.18.0/typst-x86_64-apple-darwin.tar.gz",
                    "size": 99999999
                }]
            }"#,
            )
            .create();

        let result = fetch_release_metadata_from_url(&server.url(), "latest");
        assert!(result.is_ok(), "Expected successful fetch for latest");

        let release = result.unwrap();
        assert_eq!(release.tag_name, "v0.18.0");
        assert_eq!(release.assets.len(), 1);

        mock.assert();
    }

    /// Test fetch_release_metadata with nonexistent version returns not_found_error
    #[test]
    fn test_fetch_release_metadata_nonexistent_version_returns_not_found_error() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/repos/typst/typst/releases/tags/v99.99.99")
            .with_status(404)
            .create();

        let result = fetch_release_metadata_from_url(&server.url(), "v99.99.99");
        assert!(result.is_err(), "Expected NotFound error");

        match result.unwrap_err() {
            ReleaseError::NotFound { version } => {
                assert_eq!(version, "v99.99.99");
            }
            err => panic!("Expected NotFound error, got: {:?}", err),
        }

        mock.assert();
    }

    /// Test fetch_release_metadata with rate_limit returns forbidden_error
    #[test]
    fn test_fetch_release_metadata_rate_limit_returns_forbidden_error() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/repos/typst/typst/releases/tags/v0.17.0")
            .with_status(403)
            .create();

        let result = fetch_release_metadata_from_url(&server.url(), "v0.17.0");
        assert!(result.is_err(), "Expected Forbidden error");

        match result.unwrap_err() {
            ReleaseError::Forbidden => {
                // Expected
            }
            err => panic!("Expected Forbidden error, got: {:?}", err),
        }

        mock.assert();
    }

    /// Test fetch_release_metadata with invalid_json returns invalid_json_error
    #[test]
    fn test_fetch_release_metadata_invalid_json_returns_invalid_json_error() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/repos/typst/typst/releases/tags/v0.17.0")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("not valid json")
            .create();

        let result = fetch_release_metadata_from_url(&server.url(), "v0.17.0");
        assert!(result.is_err(), "Expected JSON parse error");

        match result.unwrap_err() {
            ReleaseError::InvalidJson(_) => {
                // Expected
            }
            err => panic!("Expected InvalidJson error, got: {:?}", err),
        }

        mock.assert();
    }

    /// Test that Release struct can deserialize from GitHub API JSON
    #[test]
    fn test_release_deserialization() {
        let json = r#"{
            "tag_name": "v0.17.0",
            "assets": [
                {
                    "name": "typst-x86_64-apple-darwin.tar.gz",
                    "browser_download_url": "https://github.com/typst/typst/releases/download/v0.17.0/typst-x86_64-apple-darwin.tar.gz",
                    "size": 12345678
                }
            ]
        }"#;

        let release: Release = serde_json::from_str(json).expect("Failed to deserialize Release");
        assert_eq!(release.tag_name, "v0.17.0");
        assert_eq!(release.assets.len(), 1);
    }

    /// Test that Asset struct has correct fields
    #[test]
    fn test_asset_fields() {
        let json = r#"{
            "name": "typst-x86_64-unknown-linux-gnu.tar.gz",
            "browser_download_url": "https://example.com/typst.tar.gz",
            "size": 9876543
        }"#;

        let asset: Asset = serde_json::from_str(json).expect("Failed to deserialize Asset");
        assert_eq!(asset.name, "typst-x86_64-unknown-linux-gnu.tar.gz");
        assert_eq!(
            asset.browser_download_url.as_str(),
            "https://example.com/typst.tar.gz"
        );
        assert_eq!(asset.size, 9876543);
    }

    /// Test Release with multiple assets
    #[test]
    fn test_release_multiple_assets() {
        let json = r#"{
            "tag_name": "v0.18.0",
            "assets": [
                {
                    "name": "typst-x86_64-apple-darwin.tar.gz",
                    "browser_download_url": "https://github.com/typst/typst/releases/download/v0.18.0/darwin.tar.gz",
                    "size": 10000000
                },
                {
                    "name": "typst-x86_64-unknown-linux-gnu.tar.gz",
                    "browser_download_url": "https://github.com/typst/typst/releases/download/v0.18.0/linux.tar.gz",
                    "size": 11000000
                },
                {
                    "name": "typst-x86_64-pc-windows-msvc.zip",
                    "browser_download_url": "https://github.com/typst/typst/releases/download/v0.18.0/windows.zip",
                    "size": 12000000
                }
            ]
        }"#;

        let release: Release = serde_json::from_str(json).expect("Failed to deserialize Release");
        assert_eq!(release.tag_name, "v0.18.0");
        assert_eq!(release.assets.len(), 3);

        // Verify each asset
        assert!(
            release
                .assets
                .iter()
                .any(|a| a.name.contains("darwin") && a.size == 10000000)
        );
        assert!(
            release
                .assets
                .iter()
                .any(|a| a.name.contains("linux") && a.size == 11000000)
        );
        assert!(
            release
                .assets
                .iter()
                .any(|a| a.name.contains("windows") && a.size == 12000000)
        );
    }

    /// Test Release with no assets (edge case)
    #[test]
    fn test_release_no_assets() {
        let json = r#"{
            "tag_name": "v0.16.0",
            "assets": []
        }"#;

        let release: Release = serde_json::from_str(json).expect("Failed to deserialize Release");
        assert_eq!(release.tag_name, "v0.16.0");
        assert_eq!(release.assets.len(), 0);
    }
}

#[cfg(test)]
mod url_construction_tests {
    use super::*;

    /// Test build_release_url with valid_version constructs_correct_url
    #[test]
    fn test_build_release_url_valid_version_constructs_correct_url() {
        let url = build_release_url("https://api.github.com", "v0.17.0")
            .expect("Should construct valid URL");
        assert_eq!(
            url.as_str(),
            "https://api.github.com/repos/typst/typst/releases/tags/v0.17.0"
        );
    }

    /// Test build_release_url with latest constructs_correct_url
    #[test]
    fn test_build_release_url_latest_constructs_correct_url() {
        let url = build_release_url("https://api.github.com", "latest")
            .expect("Should construct valid URL");
        assert_eq!(
            url.as_str(),
            "https://api.github.com/repos/typst/typst/releases/latest"
        );
    }

    /// Test build_release_url with special_chars encodes_properly
    #[test]
    fn test_build_release_url_special_chars_encodes_properly() {
        let url = build_release_url("https://api.github.com", "v0.17.0-beta+metadata")
            .expect("Should handle special characters");
        assert!(url.as_str().contains("v0.17.0-beta"));
    }

    /// Test build_release_url with invalid_base_url returns_error
    #[test]
    fn test_build_release_url_invalid_base_url_returns_error() {
        let result = build_release_url("not a url", "v0.17.0");
        assert!(result.is_err(), "Should reject invalid base URL");
    }

    /// Test build_release_url with empty_version returns_error
    #[test]
    fn test_build_release_url_empty_version_returns_error() {
        let result = build_release_url("https://api.github.com", "");
        assert!(result.is_err(), "Should reject empty version");
    }

    /// Test build_release_url with path_traversal_attempt encodes_safely
    #[test]
    fn test_build_release_url_path_traversal_attempt_encodes_safely() {
        let url = build_release_url("https://api.github.com", "../../../etc/passwd")
            .expect("Should encode path traversal characters");
        // Should be encoded as part of path segment, not interpreted as traversal
        assert!(!url.as_str().contains("/etc/passwd"));
        assert!(url.as_str().contains("releases/tags/"));
    }
}
