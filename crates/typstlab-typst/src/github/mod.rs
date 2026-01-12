//! Shared GitHub interaction utilities
//!
//! This module provides common functionality for interacting with GitHub:
//! - HTTP client construction with appropriate user-agent and timeouts
//! - Generic download functionality with streaming and progress tracking
//! - Safe URL construction helpers

pub mod client;
pub mod download;
pub mod url;

// Re-exports for convenient access
pub use client::{DEFAULT_TIMEOUT, USER_AGENT, build_client, build_default_client};
pub use download::{DownloadError, DownloadOptions, download_to_memory};
pub use url::{UrlError, add_path_segments, github_api_base_url, github_base_url};
