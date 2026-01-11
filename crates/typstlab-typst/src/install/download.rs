//! Asset download and extraction functionality
//!
//! This module provides functionality to download Typst binaries from GitHub Release
//! assets and extract them to a managed cache directory. It handles:
//!
//! - Streaming HTTP downloads with progress tracking
//! - Archive extraction (.tar.xz for Unix, .zip for Windows)
//! - Binary location in extracted files (handles nested directories)
//! - Executable permissions on Unix
//! - Atomic installation to prevent corruption
//!
//! # Example
//!
//! ```no_run
//! use typstlab_typst::install::{
//!     fetch_release_metadata,
//!     select_asset_for_current_platform,
//!     download_and_install,
//!     DownloadOptions,
//! };
//! use std::path::PathBuf;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 1. Fetch release metadata
//! let release = fetch_release_metadata("v0.18.0")?;
//!
//! // 2. Select asset for current platform
//! let asset = select_asset_for_current_platform(&release)?;
//!
//! // 3. Download and install
//! let options = DownloadOptions {
//!     cache_dir: PathBuf::from("/tmp/cache"),
//!     version: "0.18.0".to_string(),
//!     progress: None,
//! };
//!
//! let binary_path = download_and_install(asset, options)?;
//! println!("Installed: {}", binary_path.display());
//! # Ok(())
//! # }
//! ```

use crate::install::release::{Asset, ReleaseError};
use std::path::{Path, PathBuf};

#[cfg(test)]
#[allow(unused_imports)]
use crate::install::platform::binary_name;

/// Download configuration options
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    /// Target cache directory (managed cache root)
    pub cache_dir: PathBuf,

    /// Version being installed (for subdirectory)
    pub version: String,

    /// Optional progress callback (bytes_downloaded, total_bytes)
    pub progress: Option<fn(u64, u64)>,
}

/// Downloads and extracts a Typst binary from a GitHub Release asset
///
/// This function performs the complete installation workflow:
/// 1. Downloads the asset from GitHub to a temporary file
/// 2. Extracts the archive to a temporary directory
/// 3. Locates the binary in the extracted files
/// 4. Sets executable permissions on Unix
/// 5. Atomically moves the binary to the cache directory
///
/// # Arguments
///
/// * `asset` - The GitHub Release asset to download
/// * `options` - Download configuration (cache directory, version, progress callback)
///
/// # Returns
///
/// Path to the installed binary (cache_dir/version/typst or typst.exe)
///
/// # Errors
///
/// Returns `ReleaseError` if:
/// - Download fails (network error, size mismatch)
/// - Extraction fails (corrupted archive, unsupported format)
/// - Binary not found in extracted files
/// - File system operations fail
///
/// # Example
///
/// ```no_run
/// use typstlab_typst::install::{download_and_install, DownloadOptions};
/// # use typstlab_typst::install::release::Asset;
/// # use url::Url;
/// # use std::path::PathBuf;
///
/// # fn example(asset: &Asset) -> Result<(), Box<dyn std::error::Error>> {
/// let options = DownloadOptions {
///     cache_dir: PathBuf::from("/tmp/cache"),
///     version: "0.18.0".to_string(),
///     progress: Some(|downloaded, total| {
///         println!("Progress: {}/{} bytes", downloaded, total);
///     }),
/// };
///
/// let binary_path = download_and_install(asset, options)?;
/// # Ok(())
/// # }
/// ```
pub fn download_and_install(
    _asset: &Asset,
    _options: DownloadOptions,
) -> Result<PathBuf, ReleaseError> {
    unimplemented!("download_and_install will be implemented in TDD green phase")
}

/// Downloads an asset to a temporary file
///
/// # Arguments
///
/// * `url` - The URL to download from
/// * `expected_size` - Expected file size for verification
/// * `progress` - Optional progress callback
///
/// # Returns
///
/// Path to the downloaded temporary file
#[allow(dead_code)]
fn download_to_temp(
    _url: &url::Url,
    _expected_size: u64,
    _progress: Option<fn(u64, u64)>,
) -> Result<PathBuf, ReleaseError> {
    unimplemented!("download_to_temp will be implemented in TDD green phase")
}

/// Extracts an archive to a temporary directory
///
/// # Arguments
///
/// * `archive_path` - Path to the archive file
/// * `archive_name` - Name of the archive (for format detection)
///
/// # Returns
///
/// Path to the temporary extraction directory
#[allow(dead_code)]
fn extract_to_temp(_archive_path: &Path, _archive_name: &str) -> Result<PathBuf, ReleaseError> {
    unimplemented!("extract_to_temp will be implemented in TDD green phase")
}

/// Extracts a .tar.xz archive
#[allow(dead_code)]
fn extract_tar(_archive_path: &Path, _dest_dir: &Path) -> Result<(), ReleaseError> {
    unimplemented!("extract_tar will be implemented in TDD green phase")
}

/// Extracts a .zip archive
#[allow(dead_code)]
fn extract_zip(_archive_path: &Path, _dest_dir: &Path) -> Result<(), ReleaseError> {
    unimplemented!("extract_zip will be implemented in TDD green phase")
}

/// Finds the binary in an extracted directory
///
/// Recursively searches for the binary using walkdir.
///
/// # Arguments
///
/// * `dir` - Directory to search in
///
/// # Returns
///
/// Path to the binary if found
#[allow(dead_code)]
fn find_binary_in_dir(_dir: &Path) -> Result<PathBuf, ReleaseError> {
    unimplemented!("find_binary_in_dir will be implemented in TDD green phase")
}

/// Sets executable permissions on Unix
#[cfg(unix)]
#[allow(dead_code)]
fn set_executable_permissions(_path: &Path) -> Result<(), ReleaseError> {
    unimplemented!("set_executable_permissions will be implemented in TDD green phase")
}

/// Atomically moves a file with ETXTBSY prevention
///
/// # Arguments
///
/// * `from` - Source path
/// * `to` - Destination path
#[allow(dead_code)]
fn atomic_move(_from: &Path, _to: &Path) -> Result<(), ReleaseError> {
    unimplemented!("atomic_move will be implemented in TDD green phase")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install::platform::binary_name;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;
    use url::Url;

    // ============================================================================
    // Test Helpers
    // ============================================================================

    /// Creates a mock Asset for testing
    fn mock_asset(name: &str, url: &str, size: u64) -> Asset {
        Asset {
            name: name.to_string(),
            browser_download_url: Url::parse(url).unwrap(),
            size,
        }
    }

    /// Creates a fake .tar.xz archive with a binary inside
    ///
    /// The binary is a shell script that prints a version string.
    fn create_fake_tar_xz_with_binary(binary_name: &str, nested: bool) -> TempDir {
        let temp = typstlab_testkit::temp_dir_in_workspace();
        let binary_content = "#!/bin/sh\necho 'typst 0.18.0'".to_string();

        // Determine platform-specific directory name
        let nested_dir_name = if cfg!(target_os = "macos") {
            if cfg!(target_arch = "x86_64") {
                "typst-x86_64-apple-darwin"
            } else {
                "typst-aarch64-apple-darwin"
            }
        } else if cfg!(target_os = "linux") {
            if cfg!(target_arch = "x86_64") {
                "typst-x86_64-unknown-linux-musl"
            } else {
                "typst-aarch64-unknown-linux-musl"
            }
        } else if cfg!(target_os = "windows") {
            if cfg!(target_arch = "x86_64") {
                "typst-x86_64-pc-windows-msvc"
            } else {
                "typst-aarch64-pc-windows-msvc"
            }
        } else {
            "typst-unknown-platform"
        };

        // Create binary in appropriate location
        let binary_path = if nested {
            let nested_dir = temp.path().join(nested_dir_name);
            fs::create_dir_all(&nested_dir).unwrap();
            nested_dir.join(binary_name)
        } else {
            temp.path().join(binary_name)
        };

        fs::write(&binary_path, binary_content).unwrap();

        // Set executable permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&binary_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&binary_path, perms).unwrap();
        }

        // Create .tar.xz archive with proper XZ compression
        let archive_path = temp.path().join("archive.tar.xz");
        let tar_xz = std::fs::File::create(&archive_path).unwrap();
        let enc = xz2::write::XzEncoder::new(tar_xz, 6);
        let mut tar = tar::Builder::new(enc);

        let binary_rel_path = if nested {
            format!("{}/{}", nested_dir_name, binary_name)
        } else {
            binary_name.to_string()
        };

        tar.append_path_with_name(&binary_path, &binary_rel_path)
            .unwrap();
        tar.finish().unwrap();

        temp
    }

    /// Creates a fake .zip archive with a binary inside
    fn create_fake_zip_with_binary(binary_name: &str, nested: bool) -> TempDir {
        let temp = typstlab_testkit::temp_dir_in_workspace();
        let binary_content = b"@echo off\r\necho typst 0.18.0";

        // Determine platform-specific directory name (typically Windows)
        let nested_dir_name = if cfg!(target_arch = "x86_64") {
            "typst-x86_64-pc-windows-msvc"
        } else {
            "typst-aarch64-pc-windows-msvc"
        };

        let archive_path = temp.path().join("archive.zip");
        let file = std::fs::File::create(&archive_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);

        let binary_rel_path = if nested {
            format!("{}/{}", nested_dir_name, binary_name)
        } else {
            binary_name.to_string()
        };

        let options: zip::write::FileOptions<'_, ()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file(&binary_rel_path, options).unwrap();
        zip.write_all(binary_content).unwrap();
        zip.finish().unwrap();

        temp
    }

    // ============================================================================
    // Download Tests
    // ============================================================================

    #[test]
    fn test_download_to_temp_success_with_size_verification() {
        // This test will verify successful download with correct size
        // Currently fails with unimplemented!()
        let url = Url::parse("https://example.com/typst.tar.xz").unwrap();
        let expected_size = 1000;
        let result = download_to_temp(&url, expected_size, None);
        assert!(result.is_ok(), "Should successfully download file");
    }

    #[test]
    fn test_download_to_temp_size_mismatch() {
        // This test will verify size mismatch error
        // Currently fails with unimplemented!()
        let url = Url::parse("https://example.com/typst.tar.xz").unwrap();
        let expected_size = 1000;
        let _actual_size = 500;
        // In real implementation, this would fail with SizeMismatch
        let result = download_to_temp(&url, expected_size, None);
        assert!(result.is_err(), "Should fail with size mismatch");
    }

    #[test]
    fn test_download_to_temp_network_error() {
        // This test will verify network error handling
        // Currently fails with unimplemented!()
        let url = Url::parse("https://invalid-url-that-does-not-exist.example/file").unwrap();
        let result = download_to_temp(&url, 1000, None);
        assert!(result.is_err(), "Should fail with network error");
    }

    #[test]
    fn test_download_to_temp_progress_callback() {
        // This test will verify progress callback invocation
        // Currently fails with unimplemented!()
        // Note: Full implementation will use mockito for HTTP mocking
        // and Arc<Mutex<Vec>> for tracking progress calls
        let url = Url::parse("https://example.com/typst.tar.xz").unwrap();

        // TODO: In green phase, use Arc<Mutex<Vec>> to track calls:
        // let progress_calls = Arc::new(Mutex::new(Vec::new()));
        // let progress_calls_clone = progress_calls.clone();
        // let progress_fn = move |downloaded: u64, total: u64| {
        //     progress_calls_clone.lock().unwrap().push((downloaded, total));
        // };

        let progress_fn = |_downloaded: u64, _total: u64| {
            // Placeholder for TDD red phase
        };
        let result = download_to_temp(&url, 1000, Some(progress_fn));
        // In green phase: assert!(!progress_calls.lock().unwrap().is_empty());
        assert!(result.is_ok(), "Should successfully download with progress");
    }

    // ============================================================================
    // Extraction Tests
    // ============================================================================

    #[test]
    fn test_extract_tar_xz_success() {
        // This test will verify successful .tar.xz extraction
        // Currently fails with unimplemented!()
        let temp = create_fake_tar_xz_with_binary("typst", false);
        let archive_path = temp.path().join("archive.tar.xz");
        let result = extract_to_temp(&archive_path, "typst-x86_64-apple-darwin.tar.xz");
        assert!(result.is_ok(), "Should successfully extract .tar.xz");
    }

    #[test]
    fn test_extract_zip_success() {
        // This test will verify successful .zip extraction
        // Currently fails with unimplemented!()
        let temp = create_fake_zip_with_binary("typst.exe", false);
        let archive_path = temp.path().join("archive.zip");
        let result = extract_to_temp(&archive_path, "typst-x86_64-pc-windows-msvc.zip");
        assert!(result.is_ok(), "Should successfully extract .zip");
    }

    #[test]
    fn test_extract_corrupted_archive() {
        // This test will verify corrupted archive error handling
        // Currently fails with unimplemented!()
        let temp = typstlab_testkit::temp_dir_in_workspace();
        let corrupted_archive = temp.path().join("corrupted.tar.xz");
        fs::write(&corrupted_archive, b"not a real archive").unwrap();

        let result = extract_to_temp(&corrupted_archive, "corrupted.tar.xz");
        assert!(
            result.is_err(),
            "Should fail with extraction error for corrupted archive"
        );
    }

    #[test]
    fn test_extract_empty_archive() {
        // This test will verify empty archive error handling
        // Currently fails with unimplemented!()
        let temp = typstlab_testkit::temp_dir_in_workspace();
        let empty_archive = temp.path().join("empty.tar.xz");

        // Create valid but empty tar.xz with proper XZ compression
        let file = std::fs::File::create(&empty_archive).unwrap();
        let enc = xz2::write::XzEncoder::new(file, 6);
        let mut tar = tar::Builder::new(enc);
        tar.finish().unwrap();

        let result = extract_to_temp(&empty_archive, "empty.tar.xz");
        // Should extract successfully but finding binary will fail
        assert!(result.is_ok(), "Should extract empty archive successfully");
    }

    // ============================================================================
    // Binary Location Tests
    // ============================================================================

    #[test]
    fn test_find_binary_at_root_level() {
        // This test will verify finding binary at root level
        // Currently fails with unimplemented!()
        let temp = typstlab_testkit::temp_dir_in_workspace();
        let binary_path = temp.path().join(binary_name());
        fs::write(&binary_path, b"fake binary").unwrap();

        let result = find_binary_in_dir(temp.path());
        assert!(result.is_ok(), "Should find binary at root level");
        assert_eq!(
            result.unwrap().file_name().unwrap(),
            binary_name(),
            "Should return correct binary path"
        );
    }

    #[test]
    fn test_find_binary_in_nested_directory() {
        // This test will verify finding binary in nested directories
        // Currently fails with unimplemented!()
        let temp = typstlab_testkit::temp_dir_in_workspace();
        let nested_dir = temp.path().join("typst-x86_64-apple-darwin");
        fs::create_dir_all(&nested_dir).unwrap();
        let binary_path = nested_dir.join(binary_name());
        fs::write(&binary_path, b"fake binary").unwrap();

        let result = find_binary_in_dir(temp.path());
        assert!(result.is_ok(), "Should find binary in nested directory");
        assert_eq!(
            result.unwrap().file_name().unwrap(),
            binary_name(),
            "Should return correct binary path"
        );
    }

    #[test]
    fn test_find_binary_not_found() {
        // This test will verify binary not found error
        // Currently fails with unimplemented!()
        let temp = typstlab_testkit::temp_dir_in_workspace();
        let result = find_binary_in_dir(temp.path());
        assert!(result.is_err(), "Should fail when binary not found");

        let err = result.unwrap_err();
        match err {
            ReleaseError::BinaryNotFoundInArchive { binary_name: _ } => {
                // Expected error type
            }
            _ => panic!("Expected BinaryNotFoundInArchive error, got: {:?}", err),
        }
    }

    // ============================================================================
    // Permissions Tests (Unix only)
    // ============================================================================

    #[test]
    #[cfg(unix)]
    fn test_set_executable_permissions() {
        // This test will verify setting executable permissions on Unix
        // Currently fails with unimplemented!()
        use std::os::unix::fs::PermissionsExt;

        let temp = typstlab_testkit::temp_dir_in_workspace();
        let binary_path = temp.path().join("typst");
        fs::write(&binary_path, b"fake binary").unwrap();

        // Initially should not be executable
        let perms_before = fs::metadata(&binary_path).unwrap().permissions();
        let mode_before = perms_before.mode();
        assert_eq!(
            mode_before & 0o111,
            0,
            "Binary should not be executable initially"
        );

        // Set executable permissions
        let result = set_executable_permissions(&binary_path);
        assert!(
            result.is_ok(),
            "Should successfully set executable permissions"
        );

        // Verify executable bit is set
        let perms_after = fs::metadata(&binary_path).unwrap().permissions();
        let mode_after = perms_after.mode();
        assert_ne!(
            mode_after & 0o111,
            0,
            "Binary should be executable after setting permissions"
        );
    }

    // ============================================================================
    // Atomic Move Tests
    // ============================================================================

    #[test]
    fn test_atomic_move_success() {
        // This test will verify successful atomic file move
        // Currently fails with unimplemented!()
        let temp = typstlab_testkit::temp_dir_in_workspace();
        let source = temp.path().join("source.bin");
        let dest = temp.path().join("dest.bin");

        fs::write(&source, b"test content").unwrap();

        let result = atomic_move(&source, &dest);
        assert!(result.is_ok(), "Should successfully move file atomically");
        assert!(dest.exists(), "Destination file should exist");
        assert!(!source.exists(), "Source file should not exist after move");
    }

    #[test]
    fn test_atomic_move_overwrite_existing() {
        // This test will verify overwriting existing file
        // Currently fails with unimplemented!()
        let temp = typstlab_testkit::temp_dir_in_workspace();
        let source = temp.path().join("source.bin");
        let dest = temp.path().join("dest.bin");

        fs::write(&source, b"new content").unwrap();
        fs::write(&dest, b"old content").unwrap();

        let result = atomic_move(&source, &dest);
        assert!(
            result.is_ok(),
            "Should successfully overwrite existing file"
        );

        let content = fs::read_to_string(&dest).unwrap();
        assert_eq!(
            content, "new content",
            "Destination should have new content"
        );
    }

    #[test]
    fn test_atomic_move_etxtbsy_prevention() {
        // This test will verify ETXTBSY prevention with sync_all
        // Currently fails with unimplemented!()
        let temp = typstlab_testkit::temp_dir_in_workspace();
        let source = temp.path().join("binary");
        let dest = temp.path().join("binary_installed");

        fs::write(&source, b"executable content").unwrap();

        let result = atomic_move(&source, &dest);
        assert!(
            result.is_ok(),
            "Should successfully move with ETXTBSY prevention"
        );

        // Verify file is ready for execution (all writes flushed)
        assert!(dest.exists(), "Destination should exist");
    }

    // ============================================================================
    // End-to-End Tests
    // ============================================================================

    #[test]
    fn test_download_and_install_complete_flow() {
        // This test will verify complete download and install flow
        // Currently fails with unimplemented!()
        let cache_dir = typstlab_testkit::temp_dir_in_workspace();
        let asset = mock_asset(
            "typst-x86_64-apple-darwin.tar.xz",
            "https://example.com/typst.tar.xz",
            1000,
        );

        let options = DownloadOptions {
            cache_dir: cache_dir.path().to_path_buf(),
            version: "0.18.0".to_string(),
            progress: None,
        };

        let result = download_and_install(&asset, options);
        assert!(
            result.is_ok(),
            "Should successfully complete download and install"
        );

        let binary_path = result.unwrap();
        assert!(
            binary_path.exists(),
            "Installed binary should exist at returned path"
        );
        assert_eq!(
            binary_path.file_name().unwrap(),
            binary_name(),
            "Binary should have correct name"
        );
    }

    #[test]
    fn test_download_and_install_creates_version_directory() {
        // This test will verify version directory creation
        // Currently fails with unimplemented!()
        let cache_dir = typstlab_testkit::temp_dir_in_workspace();
        let asset = mock_asset(
            "typst-x86_64-apple-darwin.tar.xz",
            "https://example.com/typst.tar.xz",
            1000,
        );

        let options = DownloadOptions {
            cache_dir: cache_dir.path().to_path_buf(),
            version: "0.18.0".to_string(),
            progress: None,
        };

        let result = download_and_install(&asset, options);
        assert!(result.is_ok(), "Should successfully install");

        let binary_path = result.unwrap();
        let version_dir = binary_path.parent().unwrap();
        assert!(
            version_dir.ends_with("0.18.0"),
            "Binary should be in version-specific directory"
        );
    }

    #[test]
    fn test_download_and_install_cleanup_on_error() {
        // This test will verify cleanup on errors
        // Currently fails with unimplemented!()
        let cache_dir = typstlab_testkit::temp_dir_in_workspace();
        let asset = mock_asset(
            "typst-invalid.tar.xz",
            "https://invalid-url.example/nonexistent",
            1000,
        );

        let options = DownloadOptions {
            cache_dir: cache_dir.path().to_path_buf(),
            version: "0.18.0".to_string(),
            progress: None,
        };

        let result = download_and_install(&asset, options);
        assert!(result.is_err(), "Should fail with invalid URL");

        // Verify no leftover files in cache directory
        // TempDir cleanup ensures this, but we verify explicitly
        let version_dir = cache_dir.path().join("0.18.0");
        assert!(
            !version_dir.exists() || fs::read_dir(&version_dir).unwrap().count() == 0,
            "Should not leave partial installations"
        );
    }

    #[test]
    fn test_download_and_install_with_progress_callback() {
        // This test will verify progress callback during download
        // Currently fails with unimplemented!()
        // Note: Full implementation will use Arc<Mutex<Vec>> for tracking
        let cache_dir = typstlab_testkit::temp_dir_in_workspace();
        let asset = mock_asset(
            "typst-x86_64-apple-darwin.tar.xz",
            "https://example.com/typst.tar.xz",
            1000,
        );

        // TODO: In green phase, use Arc<Mutex<Vec>> to track calls
        fn progress_callback(_downloaded: u64, _total: u64) {
            // Placeholder for TDD red phase
        }

        let options = DownloadOptions {
            cache_dir: cache_dir.path().to_path_buf(),
            version: "0.18.0".to_string(),
            progress: Some(progress_callback),
        };

        let result = download_and_install(&asset, options);
        // In green phase: verify progress_callback was invoked
        assert!(
            result.is_ok(),
            "Should successfully download with progress tracking"
        );
    }
}
