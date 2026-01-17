//! Typst documentation JSON download from typst-community/dev-builds

use crate::github;
use thiserror::Error;
use url::Url;

/// Maximum documentation JSON size (10 MB)
pub const MAX_DOCS_JSON_SIZE: u64 = 10 * 1024 * 1024;

/// Builds URL for Typst documentation JSON
///
/// Downloads from typst-community/dev-builds repository which provides
/// pre-built docs.json files for each Typst version.
///
/// # Arguments
///
/// * `version` - Typst version (e.g., "0.12.0")
///
/// # Returns
///
/// URL for downloading docs.json
///
/// # Example
///
/// ```no_run
/// # use typstlab_typst::docs::download_json::build_docs_json_url;
/// let url = build_docs_json_url("0.12.0").unwrap();
/// assert!(url.as_str().contains("typst-community"));
/// ```
pub fn build_docs_json_url(version: &str) -> Result<Url, DocsJsonError> {
    let mut url = github::github_base_url()?;

    // URL pattern: typst-community/dev-builds/releases/download/v{version}/docs.json
    let segments = &[
        "typst-community",
        "dev-builds",
        "releases",
        "download",
        &format!("v{}", version),
        "docs.json",
    ];

    github::add_path_segments(&mut url, segments)
        .map_err(|e| DocsJsonError::UrlError(e.to_string()))?;

    Ok(url)
}

/// Downloads Typst documentation JSON
///
/// # Arguments
///
/// * `version` - Typst version (e.g., "0.12.0")
/// * `verbose` - Enable verbose output
///
/// # Returns
///
/// Downloaded JSON bytes
///
/// # Errors
///
/// Returns error if:
/// - URL construction fails
/// - HTTP request fails
/// - Download size exceeds MAX_DOCS_JSON_SIZE
pub fn download_docs_json(version: &str, verbose: bool) -> Result<Vec<u8>, DocsJsonError> {
    let url = build_docs_json_url(version)?;

    if verbose {
        eprintln!("Downloading docs.json from {}...", url);
    }

    // Build client
    let client = github::build_default_client()?;

    // Download with size limit
    let options = github::DownloadOptions {
        expected_size: None,
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
    // NOTE: This check happens after download completes. Ideally, the size
    // should be enforced during download to prevent OOM from malicious/oversized
    // responses. This requires adding max_size support to github::download_to_memory().
    // For now, relying on reasonable MAX_DOCS_JSON_SIZE (10MB) and server trust.
    // TODO: Add streaming size enforcement to github/download module.
    if bytes.len() as u64 > MAX_DOCS_JSON_SIZE {
        return Err(DocsJsonError::SizeExceeded {
            size: bytes.len() as u64,
            max: MAX_DOCS_JSON_SIZE,
        });
    }

    if verbose {
        eprintln!("Downloaded {} bytes", bytes.len());
    }

    Ok(bytes)
}

/// Documentation JSON download errors
#[derive(Debug, Error)]
pub enum DocsJsonError {
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
    UrlError(String),

    /// URL parse error
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),

    /// HTTP client error
    #[error("HTTP client error: {0}")]
    HttpClientError(#[from] reqwest::Error),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    /// Test-level mutex for environment variable synchronization
    ///
    /// Prevents race conditions when tests manipulate environment variables
    /// (which are process-global) in parallel.
    static ENV_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    /// Test: URL construction for typst-community/dev-builds
    ///
    /// Verifies that build_docs_json_url() constructs valid URL pointing to
    /// typst-community/dev-builds repository with correct version.
    #[test]
    fn test_build_docs_json_url() {
        let url = build_docs_json_url("0.12.0").expect("URL construction should succeed");
        let url_str = url.as_str();

        // Should point to typst-community organization
        assert!(
            url_str.contains("typst-community"),
            "URL should contain typst-community: {}",
            url_str
        );

        // Should reference the version
        assert!(
            url_str.contains("0.12.0") || url_str.contains("v0.12.0"),
            "URL should contain version: {}",
            url_str
        );

        // Should be a docs.json file
        assert!(
            url_str.ends_with("docs.json") || url_str.contains("docs.json"),
            "URL should reference docs.json: {}",
            url_str
        );
    }

    /// Test: URL construction for different versions
    #[test]
    fn test_build_docs_json_url_multiple_versions() {
        let url_v12 = build_docs_json_url("0.12.0").expect("v0.12.0 URL should work");
        let url_v13 = build_docs_json_url("0.13.0").expect("v0.13.0 URL should work");

        assert_ne!(
            url_v12.as_str(),
            url_v13.as_str(),
            "Different versions should produce different URLs"
        );
    }

    /// Test: Size limit enforcement (10MB max)
    ///
    /// Verifies that download_docs_json() enforces MAX_DOCS_JSON_SIZE limit.
    /// Uses mock server to provide oversized response.
    #[test]
    fn test_size_limit_enforcement() {
        use mockito::Server;

        // Acquire lock to serialize environment variable access
        let _guard = ENV_TEST_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();

        let mut server = Server::new();
        let oversized_json = vec![b'x'; (MAX_DOCS_JSON_SIZE + 1) as usize];

        // Mock endpoint with oversized JSON
        let mock = server
            .mock(
                "GET",
                "/typst-community/dev-builds/releases/download/v0.12.0/docs.json",
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(&oversized_json)
            .create();

        // Override base URL to mock server
        unsafe {
            std::env::set_var("GITHUB_BASE_URL", server.url());
        }

        let result = download_docs_json("0.12.0", false);

        // Cleanup
        unsafe {
            std::env::remove_var("GITHUB_BASE_URL");
        }
        mock.assert();

        // Should reject oversized download
        assert!(result.is_err(), "Should reject oversized download");
        match result.unwrap_err() {
            DocsJsonError::SizeExceeded { size, max } => {
                assert_eq!(max, MAX_DOCS_JSON_SIZE);
                assert!(size > MAX_DOCS_JSON_SIZE);
            }
            e => panic!("Expected SizeExceeded error, got: {:?}", e),
        }
    }

    /// Test: Mock download with fixture
    ///
    /// Verifies that download_docs_json() successfully downloads valid JSON
    /// from mock server.
    #[test]
    fn test_mock_download_success() {
        use mockito::Server;

        // Acquire lock to serialize environment variable access
        let _guard = ENV_TEST_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();

        let mut server = Server::new();
        let fixture_json = b"{\"route\":\"/\",\"title\":\"Documentation\",\"children\":[]}";

        // Mock endpoint with valid JSON
        let mock = server
            .mock(
                "GET",
                "/typst-community/dev-builds/releases/download/v0.12.0/docs.json",
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(fixture_json)
            .create();

        // Override base URL to mock server
        unsafe {
            std::env::set_var("GITHUB_BASE_URL", server.url());
        }

        let result = download_docs_json("0.12.0", false);

        // Cleanup
        unsafe {
            std::env::remove_var("GITHUB_BASE_URL");
        }
        mock.assert();

        // Should succeed
        let bytes = result.expect("Download should succeed");
        assert_eq!(bytes, fixture_json);
    }

    /// Test: Verbose output
    ///
    /// Verifies that verbose flag enables progress output.
    /// This test only checks that verbose mode doesn't crash.
    #[test]
    fn test_verbose_output() {
        use mockito::Server;

        // Acquire lock to serialize environment variable access
        let _guard = ENV_TEST_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();

        let mut server = Server::new();
        let fixture_json = b"{}";

        let mock = server
            .mock(
                "GET",
                "/typst-community/dev-builds/releases/download/v0.12.0/docs.json",
            )
            .with_status(200)
            .with_body(fixture_json)
            .create();

        unsafe {
            std::env::set_var("GITHUB_BASE_URL", server.url());
        }

        // Should not panic with verbose=true
        let result = download_docs_json("0.12.0", true);

        unsafe {
            std::env::remove_var("GITHUB_BASE_URL");
        }
        mock.assert();

        assert!(result.is_ok(), "Verbose download should succeed");
    }
}
