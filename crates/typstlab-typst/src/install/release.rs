//! GitHub Release binary download and installation
//!
//! This module handles downloading pre-built Typst binaries from GitHub Releases.

use serde::{Deserialize, Serialize};

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
    pub browser_download_url: String,
    /// File size in bytes
    pub size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

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
            asset.browser_download_url,
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
