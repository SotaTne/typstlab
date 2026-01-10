//! Asset selection for platform-specific binary installation
//!
//! This module provides functionality to select the appropriate Typst binary asset
//! from a GitHub Release based on the current platform (OS and architecture).
//!
//! # Platform Detection
//!
//! Uses the `platform` module extensively for:
//! - OS detection (`detect_os`)
//! - Architecture detection (`detect_arch`)
//! - Asset name pattern generation (`asset_name_pattern`)
//!
//! # Example
//!
//! ```no_run
//! use typstlab_typst::install::{fetch_release_metadata, select_asset_for_current_platform};
//!
//! let release = fetch_release_metadata("v0.18.0")?;
//! let asset = select_asset_for_current_platform(&release)?;
//! println!("Download URL: {}", asset.browser_download_url);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::install::platform::{Arch, Os, asset_name_pattern, detect_arch, detect_os};
use crate::install::release::{Asset, Release, ReleaseError};

/// Selects the appropriate asset from a Release based on OS and architecture.
///
/// Searches the release assets for an asset whose filename contains the platform
/// pattern generated from the given OS and architecture.
///
/// # Arguments
///
/// * `release` - The Release metadata containing available assets
/// * `os` - The target operating system
/// * `arch` - The target architecture
///
/// # Returns
///
/// - `Ok(&Asset)` if a matching asset is found
/// - `Err(ReleaseError::AssetNotFound)` if no matching asset exists
///
/// # Platform Patterns
///
/// The function uses these patterns for matching:
/// - MacOS x86_64: "x86_64-apple-darwin"
/// - MacOS Aarch64: "aarch64-apple-darwin"
/// - Linux x86_64: "x86_64-unknown-linux"
/// - Linux Aarch64: "aarch64-unknown-linux"
/// - Windows x86_64: "x86_64-pc-windows"
/// - Windows Aarch64: "aarch64-pc-windows"
///
/// # Examples
///
/// ```no_run
/// use typstlab_typst::install::{fetch_release_metadata, select_asset, Os, Arch};
///
/// let release = fetch_release_metadata("v0.18.0")?;
/// let asset = select_asset(&release, Os::MacOS, Arch::X86_64)?;
/// println!("Download URL: {}", asset.browser_download_url);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn select_asset(_release: &Release, _os: Os, _arch: Arch) -> Result<&Asset, ReleaseError> {
    unimplemented!("TDD red phase: select_asset not yet implemented")
}

/// Convenience function that automatically detects the current platform
/// and selects the matching asset from a Release.
///
/// This is the typical entry point for binary installation. It combines:
/// 1. Platform OS detection (using `platform::detect_os`)
/// 2. Architecture detection (using `platform::detect_arch`)
/// 3. Asset selection (using `select_asset`)
///
/// # Arguments
///
/// * `release` - The Release metadata containing available assets
///
/// # Returns
///
/// - `Ok(&Asset)` if current platform is supported and asset found
/// - `Err(ReleaseError::*)` if:
///   - OS cannot be detected (unsupported platform)
///   - Architecture cannot be detected (unsupported platform)
///   - No matching asset in release for detected platform
///
/// # Examples
///
/// ```no_run
/// use typstlab_typst::install::{fetch_release_metadata, select_asset_for_current_platform};
///
/// let release = fetch_release_metadata("v0.18.0")?;
/// let asset = select_asset_for_current_platform(&release)?;
/// println!("Download: {}", asset.browser_download_url);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn select_asset_for_current_platform(_release: &Release) -> Result<&Asset, ReleaseError> {
    unimplemented!("TDD red phase: select_asset_for_current_platform not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    /// Helper: Creates a mock release with assets for all major platforms
    fn create_mock_release_multiplatform() -> Release {
        Release {
            tag_name: "v0.18.0".to_string(),
            assets: vec![
                Asset {
                    name: "typst-x86_64-apple-darwin.tar.gz".to_string(),
                    browser_download_url: Url::parse("https://example.com/darwin.tar.gz").unwrap(),
                    size: 10000000,
                },
                Asset {
                    name: "typst-aarch64-apple-darwin.tar.gz".to_string(),
                    browser_download_url: Url::parse("https://example.com/darwin-arm.tar.gz")
                        .unwrap(),
                    size: 10000000,
                },
                Asset {
                    name: "typst-x86_64-unknown-linux-gnu.tar.gz".to_string(),
                    browser_download_url: Url::parse("https://example.com/linux.tar.gz").unwrap(),
                    size: 11000000,
                },
                Asset {
                    name: "typst-aarch64-unknown-linux-gnu.tar.gz".to_string(),
                    browser_download_url: Url::parse("https://example.com/linux-arm.tar.gz")
                        .unwrap(),
                    size: 11000000,
                },
                Asset {
                    name: "typst-x86_64-pc-windows-msvc.zip".to_string(),
                    browser_download_url: Url::parse("https://example.com/windows.zip").unwrap(),
                    size: 12000000,
                },
                Asset {
                    name: "typst-aarch64-pc-windows-msvc.zip".to_string(),
                    browser_download_url: Url::parse("https://example.com/windows-arm.zip")
                        .unwrap(),
                    size: 12000000,
                },
            ],
        }
    }

    // ========================================================================
    // Core Selection Tests - All 6 Platform Combinations
    // ========================================================================

    #[test]
    fn test_select_asset_macos_x86_64_returns_correct_asset() {
        let release = create_mock_release_multiplatform();
        let asset = select_asset(&release, Os::MacOS, Arch::X86_64).unwrap();
        assert!(
            asset.name.contains("x86_64-apple-darwin"),
            "Asset name should contain MacOS x86_64 pattern"
        );
        assert_eq!(asset.size, 10000000);
    }

    #[test]
    fn test_select_asset_macos_aarch64_returns_correct_asset() {
        let release = create_mock_release_multiplatform();
        let asset = select_asset(&release, Os::MacOS, Arch::Aarch64).unwrap();
        assert!(
            asset.name.contains("aarch64-apple-darwin"),
            "Asset name should contain MacOS aarch64 pattern"
        );
    }

    #[test]
    fn test_select_asset_linux_x86_64_returns_correct_asset() {
        let release = create_mock_release_multiplatform();
        let asset = select_asset(&release, Os::Linux, Arch::X86_64).unwrap();
        assert!(
            asset.name.contains("x86_64-unknown-linux"),
            "Asset name should contain Linux x86_64 pattern"
        );
        assert_eq!(asset.size, 11000000);
    }

    #[test]
    fn test_select_asset_linux_aarch64_returns_correct_asset() {
        let release = create_mock_release_multiplatform();
        let asset = select_asset(&release, Os::Linux, Arch::Aarch64).unwrap();
        assert!(
            asset.name.contains("aarch64-unknown-linux"),
            "Asset name should contain Linux aarch64 pattern"
        );
    }

    #[test]
    fn test_select_asset_windows_x86_64_returns_correct_asset() {
        let release = create_mock_release_multiplatform();
        let asset = select_asset(&release, Os::Windows, Arch::X86_64).unwrap();
        assert!(
            asset.name.contains("x86_64-pc-windows"),
            "Asset name should contain Windows x86_64 pattern"
        );
        assert_eq!(asset.size, 12000000);
    }

    #[test]
    fn test_select_asset_windows_aarch64_returns_correct_asset() {
        let release = create_mock_release_multiplatform();
        let asset = select_asset(&release, Os::Windows, Arch::Aarch64).unwrap();
        assert!(
            asset.name.contains("aarch64-pc-windows"),
            "Asset name should contain Windows aarch64 pattern"
        );
    }

    // ========================================================================
    // Edge Cases - Error Paths
    // ========================================================================

    #[test]
    fn test_select_asset_no_matching_asset_returns_error() {
        // Release with only Linux assets
        let release = Release {
            tag_name: "v0.18.0".to_string(),
            assets: vec![Asset {
                name: "typst-x86_64-unknown-linux-gnu.tar.gz".to_string(),
                browser_download_url: Url::parse("https://example.com/linux.tar.gz").unwrap(),
                size: 11000000,
            }],
        };

        let result = select_asset(&release, Os::MacOS, Arch::X86_64);
        assert!(
            result.is_err(),
            "Should return error when no matching asset found"
        );

        match result.unwrap_err() {
            ReleaseError::AssetNotFound { version, os, arch } => {
                assert_eq!(version, "v0.18.0");
                assert!(os.contains("MacOS") || os.contains("macos"));
                assert!(arch.contains("X86_64") || arch.contains("x86_64"));
            }
            err => panic!("Expected AssetNotFound error, got: {:?}", err),
        }
    }

    #[test]
    fn test_select_asset_empty_asset_list_returns_error() {
        let release = Release {
            tag_name: "v0.16.0".to_string(),
            assets: vec![],
        };

        let result = select_asset(&release, Os::Linux, Arch::X86_64);
        assert!(
            result.is_err(),
            "Should return error when asset list is empty"
        );
    }

    #[test]
    fn test_select_asset_multiple_matching_returns_first() {
        // Release with duplicate matching assets (unlikely but possible)
        let release = Release {
            tag_name: "v0.18.0".to_string(),
            assets: vec![
                Asset {
                    name: "typst-x86_64-apple-darwin.tar.gz".to_string(),
                    browser_download_url: Url::parse("https://example.com/first.tar.gz").unwrap(),
                    size: 10000000,
                },
                Asset {
                    name: "typst-x86_64-apple-darwin-alternative.tar.gz".to_string(),
                    browser_download_url: Url::parse("https://example.com/second.tar.gz").unwrap(),
                    size: 10000001,
                },
            ],
        };

        let asset = select_asset(&release, Os::MacOS, Arch::X86_64).unwrap();
        // Should return first match
        assert_eq!(
            asset.browser_download_url.as_str(),
            "https://example.com/first.tar.gz"
        );
        assert_eq!(asset.size, 10000000);
    }

    // ========================================================================
    // Platform Auto-Detection Tests
    // ========================================================================

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    fn test_select_asset_for_current_platform_macos_x86() {
        let release = create_mock_release_multiplatform();
        let asset = select_asset_for_current_platform(&release).unwrap();
        assert!(
            asset.name.contains("darwin"),
            "Should select MacOS asset on MacOS platform"
        );
        assert!(
            asset.name.contains("x86_64"),
            "Should select x86_64 asset on x86_64 architecture"
        );
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn test_select_asset_for_current_platform_macos_aarch64() {
        let release = create_mock_release_multiplatform();
        let asset = select_asset_for_current_platform(&release).unwrap();
        assert!(
            asset.name.contains("darwin"),
            "Should select MacOS asset on MacOS platform"
        );
        assert!(
            asset.name.contains("aarch64"),
            "Should select aarch64 asset on aarch64 architecture"
        );
    }

    #[test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn test_select_asset_for_current_platform_linux_x86() {
        let release = create_mock_release_multiplatform();
        let asset = select_asset_for_current_platform(&release).unwrap();
        assert!(
            asset.name.contains("linux"),
            "Should select Linux asset on Linux platform"
        );
        assert!(
            asset.name.contains("x86_64"),
            "Should select x86_64 asset on x86_64 architecture"
        );
    }

    #[test]
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    fn test_select_asset_for_current_platform_linux_aarch64() {
        let release = create_mock_release_multiplatform();
        let asset = select_asset_for_current_platform(&release).unwrap();
        assert!(
            asset.name.contains("linux"),
            "Should select Linux asset on Linux platform"
        );
        assert!(
            asset.name.contains("aarch64"),
            "Should select aarch64 asset on aarch64 architecture"
        );
    }

    #[test]
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    fn test_select_asset_for_current_platform_windows_x86() {
        let release = create_mock_release_multiplatform();
        let asset = select_asset_for_current_platform(&release).unwrap();
        assert!(
            asset.name.contains("windows"),
            "Should select Windows asset on Windows platform"
        );
        assert!(
            asset.name.contains("x86_64"),
            "Should select x86_64 asset on x86_64 architecture"
        );
    }

    #[test]
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    fn test_select_asset_for_current_platform_windows_aarch64() {
        let release = create_mock_release_multiplatform();
        let asset = select_asset_for_current_platform(&release).unwrap();
        assert!(
            asset.name.contains("windows"),
            "Should select Windows asset on Windows platform"
        );
        assert!(
            asset.name.contains("aarch64"),
            "Should select aarch64 asset on aarch64 architecture"
        );
    }

    // ========================================================================
    // Real-World Data Tests
    // ========================================================================

    #[test]
    fn test_select_asset_with_real_typst_release_schema() {
        // Mimics actual GitHub Release JSON structure
        let release = Release {
            tag_name: "v0.17.0".to_string(),
            assets: vec![
                Asset {
                    name: "typst-x86_64-apple-darwin.tar.gz".to_string(),
                    browser_download_url: Url::parse(
                        "https://github.com/typst/typst/releases/download/v0.17.0/typst-x86_64-apple-darwin.tar.gz",
                    )
                    .unwrap(),
                    size: 8675309,
                },
                Asset {
                    name: "typst-x86_64-unknown-linux-gnu.tar.gz".to_string(),
                    browser_download_url: Url::parse(
                        "https://github.com/typst/typst/releases/download/v0.17.0/typst-x86_64-unknown-linux-gnu.tar.gz",
                    )
                    .unwrap(),
                    size: 8765432,
                },
                Asset {
                    name: "typst-x86_64-pc-windows-msvc.zip".to_string(),
                    browser_download_url: Url::parse(
                        "https://github.com/typst/typst/releases/download/v0.17.0/typst-x86_64-pc-windows-msvc.zip",
                    )
                    .unwrap(),
                    size: 9123456,
                },
            ],
        };

        // Test each platform
        let macos_asset = select_asset(&release, Os::MacOS, Arch::X86_64).unwrap();
        assert_eq!(macos_asset.name, "typst-x86_64-apple-darwin.tar.gz");
        assert!(
            macos_asset
                .browser_download_url
                .as_str()
                .contains("github.com")
        );

        let linux_asset = select_asset(&release, Os::Linux, Arch::X86_64).unwrap();
        assert_eq!(linux_asset.name, "typst-x86_64-unknown-linux-gnu.tar.gz");

        let windows_asset = select_asset(&release, Os::Windows, Arch::X86_64).unwrap();
        assert_eq!(windows_asset.name, "typst-x86_64-pc-windows-msvc.zip");
    }

    #[test]
    fn test_select_asset_preserves_download_url() {
        let release = create_mock_release_multiplatform();
        let asset = select_asset(&release, Os::MacOS, Arch::X86_64).unwrap();

        // Url type is already validated by deserializer
        assert_eq!(
            asset.browser_download_url.as_str(),
            "https://example.com/darwin.tar.gz"
        );

        // Verify it's a valid URL
        assert!(asset.browser_download_url.scheme() == "https");
        assert!(asset.browser_download_url.host_str().is_some());
    }
}
