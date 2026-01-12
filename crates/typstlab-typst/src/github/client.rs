//! HTTP client construction for GitHub interactions

use reqwest::blocking::Client;
use std::time::Duration;

/// Default timeout for GitHub requests (30 seconds)
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Default user agent for typstlab requests
pub const USER_AGENT: &str = "typstlab";

/// Builds HTTP client with appropriate settings for GitHub
///
/// # Arguments
///
/// * `timeout` - Request timeout duration
///
/// # Returns
///
/// Configured HTTP client
///
/// # Errors
///
/// Returns error if client construction fails
pub fn build_client(timeout: Duration) -> Result<Client, reqwest::Error> {
    Client::builder()
        .user_agent(USER_AGENT)
        .timeout(timeout)
        .build()
}

/// Builds HTTP client with default timeout
///
/// # Returns
///
/// Configured HTTP client with DEFAULT_TIMEOUT
///
/// # Errors
///
/// Returns error if client construction fails
pub fn build_default_client() -> Result<Client, reqwest::Error> {
    build_client(DEFAULT_TIMEOUT)
}
