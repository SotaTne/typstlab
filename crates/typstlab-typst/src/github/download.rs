//! Generic download functionality for GitHub resources

use reqwest::blocking::Client;
use std::io::Read;
use std::time::Duration;
use thiserror::Error;
use url::Url;

/// Download options
#[derive(Debug, Clone, Default)]
pub struct DownloadOptions {
    /// Optional progress callback (bytes_downloaded, total_bytes)
    pub progress: Option<fn(u64, u64)>,

    /// Optional expected size for verification
    pub expected_size: Option<u64>,

    /// Client timeout override (if None, uses client's default)
    pub timeout: Option<Duration>,
}

/// Downloads from URL to memory with streaming and size verification
///
/// # Arguments
///
/// * `client` - HTTP client to use
/// * `url` - URL to download from
/// * `options` - Download configuration
///
/// # Returns
///
/// Downloaded bytes as Vec<u8>
///
/// # Errors
///
/// Returns error if:
/// - HTTP request fails
/// - Response status is not success
/// - Size mismatch (if expected_size provided)
/// - I/O error during download
pub fn download_to_memory(
    client: &Client,
    url: &Url,
    options: DownloadOptions,
) -> Result<Vec<u8>, DownloadError> {
    // Send GET request
    let mut response = client.get(url.as_str()).send()?;

    // Check status
    if let Err(err) = response.error_for_status_ref() {
        return Err(DownloadError::HttpError {
            url: url.clone(),
            source: err,
        });
    }

    // Get content length for verification and progress
    let content_length = response.content_length();

    // Stream download with progress tracking
    let mut buffer = Vec::new();
    let mut chunk = [0; 8192];
    let mut downloaded: u64 = 0;

    loop {
        let bytes_read = response.read(&mut chunk)?;
        if bytes_read == 0 {
            break;
        }

        buffer.extend_from_slice(&chunk[..bytes_read]);
        downloaded += bytes_read as u64;

        // Invoke progress callback
        if let Some(callback) = options.progress {
            let total = content_length.unwrap_or(downloaded);
            callback(downloaded, total);
        }
    }

    // Verify size if expected
    if let Some(expected) = options.expected_size
        && downloaded != expected
    {
        return Err(DownloadError::SizeMismatch {
            expected,
            actual: downloaded,
        });
    }

    Ok(buffer)
}

/// Download error types
#[derive(Debug, Error)]
pub enum DownloadError {
    /// HTTP error during download
    #[error("HTTP error downloading {url}: {source}")]
    HttpError {
        /// URL that failed
        url: Url,
        /// Underlying reqwest error
        #[source]
        source: reqwest::Error,
    },

    /// Downloaded size does not match expected size
    #[error("Size mismatch: expected {expected} bytes, got {actual} bytes")]
    SizeMismatch {
        /// Expected size in bytes
        expected: u64,
        /// Actual downloaded size in bytes
        actual: u64,
    },

    /// I/O error during download
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
}
