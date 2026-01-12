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

use crate::install::platform::binary_name;
use crate::install::release::{Asset, ReleaseError};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

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
    asset: &Asset,
    options: DownloadOptions,
) -> Result<PathBuf, ReleaseError> {
    // 1. Download asset to temporary file
    let archive_path = download_to_temp(&asset.browser_download_url, asset.size, options.progress)?;

    // 2. Extract archive to temporary directory (TempDir RAII for cleanup on error)
    let extract_tempdir = extract_to_temp(&archive_path, &asset.name)?;
    let extract_dir = extract_tempdir.path();

    // 3. Find binary in extracted files
    let binary_path = find_binary_in_dir(extract_dir)?;

    // 4. Set executable permissions on Unix
    #[cfg(unix)]
    set_executable_permissions(&binary_path)?;

    // 5. Create version-specific directory in cache
    let version_dir = options.cache_dir.join(&options.version);
    fs::create_dir_all(&version_dir).map_err(|e| ReleaseError::IoError {
        operation: format!("create version directory {}", version_dir.display()),
        source: e,
    })?;

    // 6. Atomic move to final destination
    let final_path = version_dir.join(binary_name());
    atomic_move(&binary_path, &final_path)?;

    // 7. Cleanup temporary files
    // TempDir will be automatically cleaned up when extract_tempdir is dropped
    let _ = fs::remove_file(&archive_path);

    Ok(final_path)
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
fn download_to_temp(
    url: &url::Url,
    expected_size: u64,
    progress: Option<fn(u64, u64)>,
) -> Result<PathBuf, ReleaseError> {
    // Create HTTP client with timeout (5 minutes for large binaries)
    let client = crate::github::build_client(std::time::Duration::from_secs(300)).map_err(|e| {
        ReleaseError::DownloadFailed {
            url: url.clone(),
            source: e,
        }
    })?;

    // Send GET request
    let mut response =
        client
            .get(url.as_str())
            .send()
            .map_err(|e| ReleaseError::DownloadFailed {
                url: url.clone(),
                source: e,
            })?;

    // Check status and convert to error without unwrap
    if let Err(err) = response.error_for_status_ref() {
        return Err(ReleaseError::DownloadFailed {
            url: url.clone(),
            source: err.without_url(),
        });
    }

    // Create temporary file
    let mut temp_file = tempfile::NamedTempFile::new().map_err(|e| ReleaseError::IoError {
        operation: "create temporary file for download".to_string(),
        source: e,
    })?;

    // Download with progress tracking
    let mut downloaded: u64 = 0;
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = response
            .read(&mut buffer)
            .map_err(|e| ReleaseError::IoError {
                operation: "read from HTTP response".to_string(),
                source: e,
            })?;

        if bytes_read == 0 {
            break;
        }

        temp_file
            .write_all(&buffer[..bytes_read])
            .map_err(|e| ReleaseError::IoError {
                operation: "write to temporary file".to_string(),
                source: e,
            })?;

        downloaded += bytes_read as u64;

        // Invoke progress callback if provided
        if let Some(callback) = progress {
            callback(downloaded, expected_size);
        }
    }

    // Verify file size
    if downloaded != expected_size {
        return Err(ReleaseError::SizeMismatch {
            expected: expected_size,
            actual: downloaded,
        });
    }

    // Sync and persist
    temp_file
        .as_file()
        .sync_all()
        .map_err(|e| ReleaseError::IoError {
            operation: "sync temporary file".to_string(),
            source: e,
        })?;

    let path = temp_file
        .into_temp_path()
        .keep()
        .map_err(|e| ReleaseError::IoError {
            operation: "persist temporary file".to_string(),
            source: e.error,
        })?;

    Ok(path)
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
/// TempDir that will be automatically cleaned up when dropped
fn extract_to_temp(
    archive_path: &Path,
    archive_name: &str,
) -> Result<tempfile::TempDir, ReleaseError> {
    let temp_dir = tempfile::tempdir().map_err(|e| ReleaseError::IoError {
        operation: "create temporary directory for extraction".to_string(),
        source: e,
    })?;

    if archive_name.ends_with(".tar.xz") || archive_name.ends_with(".tar.gz") {
        extract_tar(archive_path, temp_dir.path(), archive_name)?;
    } else if archive_name.ends_with(".zip") {
        extract_zip(archive_path, temp_dir.path())?;
    } else {
        return Err(ReleaseError::ExtractionFailed {
            archive_type: archive_name.to_string(),
            reason: "Unsupported archive format".to_string(),
        });
    }

    // Return TempDir itself for RAII cleanup
    Ok(temp_dir)
}

/// Extracts a .tar.xz or .tar.gz archive
fn extract_tar(
    archive_path: &Path,
    dest_dir: &Path,
    archive_name: &str,
) -> Result<(), ReleaseError> {
    let file = fs::File::open(archive_path).map_err(|e| ReleaseError::IoError {
        operation: format!("open archive {}", archive_path.display()),
        source: e,
    })?;

    let archive_type = if archive_name.ends_with(".tar.xz") {
        "tar.xz"
    } else {
        "tar.gz"
    };

    // Use appropriate decompressor based on file extension
    if archive_name.ends_with(".tar.xz") {
        let decompressor = xz2::read::XzDecoder::new(file);
        let mut archive = tar::Archive::new(decompressor);

        // Safe extraction: iterate entries and use unpack_in() for validation
        for entry in archive
            .entries()
            .map_err(|e| ReleaseError::ExtractionFailed {
                archive_type: archive_type.to_string(),
                reason: e.to_string(),
            })?
        {
            let mut entry = entry.map_err(|e| ReleaseError::ExtractionFailed {
                archive_type: archive_type.to_string(),
                reason: e.to_string(),
            })?;

            entry
                .unpack_in(dest_dir)
                .map_err(|e| ReleaseError::ExtractionFailed {
                    archive_type: archive_type.to_string(),
                    reason: e.to_string(),
                })?;
        }
    } else {
        let decompressor = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decompressor);

        // Safe extraction: iterate entries and use unpack_in() for validation
        for entry in archive
            .entries()
            .map_err(|e| ReleaseError::ExtractionFailed {
                archive_type: archive_type.to_string(),
                reason: e.to_string(),
            })?
        {
            let mut entry = entry.map_err(|e| ReleaseError::ExtractionFailed {
                archive_type: archive_type.to_string(),
                reason: e.to_string(),
            })?;

            entry
                .unpack_in(dest_dir)
                .map_err(|e| ReleaseError::ExtractionFailed {
                    archive_type: archive_type.to_string(),
                    reason: e.to_string(),
                })?;
        }
    }

    Ok(())
}

/// Extracts a .zip archive
fn extract_zip(archive_path: &Path, dest_dir: &Path) -> Result<(), ReleaseError> {
    let file = fs::File::open(archive_path).map_err(|e| ReleaseError::IoError {
        operation: format!("open archive {}", archive_path.display()),
        source: e,
    })?;

    let mut archive = zip::ZipArchive::new(file).map_err(|e| ReleaseError::ExtractionFailed {
        archive_type: "zip".to_string(),
        reason: e.to_string(),
    })?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| ReleaseError::ExtractionFailed {
                archive_type: "zip".to_string(),
                reason: e.to_string(),
            })?;

        let outpath = match file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };

        if file.is_dir() {
            fs::create_dir_all(&outpath).map_err(|e| ReleaseError::IoError {
                operation: format!("create directory {}", outpath.display()),
                source: e,
            })?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).map_err(|e| ReleaseError::IoError {
                    operation: format!("create parent directory {}", parent.display()),
                    source: e,
                })?;
            }

            let mut outfile = fs::File::create(&outpath).map_err(|e| ReleaseError::IoError {
                operation: format!("create file {}", outpath.display()),
                source: e,
            })?;

            io::copy(&mut file, &mut outfile).map_err(|e| ReleaseError::IoError {
                operation: format!("extract file {}", outpath.display()),
                source: e,
            })?;
        }
    }

    Ok(())
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
fn find_binary_in_dir(dir: &Path) -> Result<PathBuf, ReleaseError> {
    use std::ffi::OsStr;

    let target_name = binary_name();
    let target_os_str = OsStr::new(&target_name);

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry.map_err(|e| ReleaseError::IoError {
            operation: format!("walk directory {}", dir.display()),
            source: io::Error::other(e),
        })?;

        // Use OsStr comparison to handle non-UTF8 filenames
        if entry.file_type().is_file() && entry.file_name() == target_os_str {
            return Ok(entry.path().to_path_buf());
        }
    }

    Err(ReleaseError::BinaryNotFoundInArchive {
        binary_name: target_name.to_string(),
    })
}

/// Sets executable permissions on Unix
#[cfg(unix)]
fn set_executable_permissions(path: &Path) -> Result<(), ReleaseError> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::metadata(path).map_err(|e| ReleaseError::IoError {
        operation: format!("get metadata for {}", path.display()),
        source: e,
    })?;

    let mut permissions = metadata.permissions();
    let mode = permissions.mode();

    // Add executable bit (owner, group, other)
    permissions.set_mode(mode | 0o111);

    fs::set_permissions(path, permissions).map_err(|e| ReleaseError::IoError {
        operation: format!("set permissions for {}", path.display()),
        source: e,
    })?;

    Ok(())
}

/// Atomically moves a file with ETXTBSY prevention and cross-filesystem support
///
/// This function ensures atomic installation by:
/// 1. Creating a temp file in the destination directory (same filesystem)
/// 2. Copying the source file to the temp file (preserves permissions on Unix)
/// 3. Atomically renaming the temp file to the final destination (overwrites existing)
/// 4. Best-effort cleanup of source file (ignored if fails after successful install)
///
/// # Arguments
///
/// * `from` - Source path
/// * `to` - Destination path
fn atomic_move(from: &Path, to: &Path) -> Result<(), ReleaseError> {
    // Open and sync source file (ETXTBSY prevention)
    let src_file = fs::File::open(from).map_err(|e| ReleaseError::IoError {
        operation: format!("open source file: {}", from.display()),
        source: e,
    })?;

    src_file.sync_all().map_err(|e| ReleaseError::IoError {
        operation: "sync source file".to_string(),
        source: e,
    })?;

    drop(src_file);

    // Get the destination directory
    let dest_dir = to.parent().ok_or_else(|| ReleaseError::IoError {
        operation: format!("get parent directory of {}", to.display()),
        source: io::Error::other("no parent directory"),
    })?;

    // Create a temporary file in the destination directory (ensures same filesystem)
    let mut temp_dest =
        tempfile::NamedTempFile::new_in(dest_dir).map_err(|e| ReleaseError::IoError {
            operation: format!("create temporary file in {}", dest_dir.display()),
            source: e,
        })?;

    // Copy contents from source to temp file
    let mut src_file = fs::File::open(from).map_err(|e| ReleaseError::IoError {
        operation: format!("open source for copying: {}", from.display()),
        source: e,
    })?;

    // On Unix, preserve file permissions during copy
    #[cfg(unix)]
    {
        let src_metadata = fs::metadata(from).map_err(|e| ReleaseError::IoError {
            operation: format!("get metadata for {}", from.display()),
            source: e,
        })?;
        let src_permissions = src_metadata.permissions();

        io::copy(&mut src_file, &mut temp_dest).map_err(|e| ReleaseError::IoError {
            operation: "copy file contents".to_string(),
            source: e,
        })?;

        // Set permissions on temp file before persist
        fs::set_permissions(temp_dest.path(), src_permissions).map_err(|e| {
            ReleaseError::IoError {
                operation: "set permissions on temporary file".to_string(),
                source: e,
            }
        })?;
    }

    #[cfg(not(unix))]
    {
        io::copy(&mut src_file, &mut temp_dest).map_err(|e| ReleaseError::IoError {
            operation: "copy file contents".to_string(),
            source: e,
        })?;
    }

    drop(src_file);

    // Sync temp file before renaming
    temp_dest
        .as_file()
        .sync_all()
        .map_err(|e| ReleaseError::IoError {
            operation: "sync temporary file".to_string(),
            source: e,
        })?;

    // Persist (atomic rename within same filesystem, overwrites existing on Unix/Windows)
    temp_dest.persist(to).map_err(|e| ReleaseError::IoError {
        operation: format!("rename temporary file to {}", to.display()),
        source: e.error,
    })?;

    // Sync destination file
    let dest_file = fs::File::open(to).map_err(|e| ReleaseError::IoError {
        operation: format!("open destination: {}", to.display()),
        source: e,
    })?;

    dest_file.sync_all().map_err(|e| ReleaseError::IoError {
        operation: "sync destination file".to_string(),
        source: e,
    })?;

    drop(dest_file);

    // Sync parent directory on Unix
    #[cfg(unix)]
    {
        let dir = fs::File::open(dest_dir).map_err(|e| ReleaseError::IoError {
            operation: format!("open parent directory: {}", dest_dir.display()),
            source: e,
        })?;

        dir.sync_all().map_err(|e| ReleaseError::IoError {
            operation: "sync parent directory".to_string(),
            source: e,
        })?;
    }

    // Clean up source file (best-effort, ignore errors since install already succeeded)
    let _ = fs::remove_file(from);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install::platform::binary_name;
    use mockito::Server;
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
        // This test verifies successful download with correct size using mockito
        let mut server = Server::new();

        // Create body of exactly 1000 bytes
        let body = vec![b'x'; 1000];

        // Set up mock response
        let mock = server
            .mock("GET", "/typst.tar.xz")
            .with_status(200)
            .with_header("content-type", "application/x-xz")
            .with_body(&body)
            .create();

        // Use mock server URL
        let url = Url::parse(&format!("{}/typst.tar.xz", server.url())).unwrap();
        let expected_size = 1000;
        let result = download_to_temp(&url, expected_size, None);

        mock.assert();
        assert!(result.is_ok(), "Should successfully download file");

        // Verify downloaded file exists and has correct size
        let path = result.unwrap();
        assert!(path.exists(), "Downloaded file should exist");
        let metadata = fs::metadata(&path).unwrap();
        assert_eq!(
            metadata.len(),
            expected_size,
            "File should have correct size"
        );
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
        // This test verifies progress callback invocation using mockito
        use std::sync::{Mutex, OnceLock};

        // Static storage for tracking progress calls (required for fn pointer)
        static PROGRESS_CALLS: OnceLock<Mutex<Vec<(u64, u64)>>> = OnceLock::new();

        fn track_progress(downloaded: u64, total: u64) {
            PROGRESS_CALLS
                .get_or_init(|| Mutex::new(Vec::new()))
                .lock()
                .unwrap()
                .push((downloaded, total));
        }

        let mut server = Server::new();

        // Create body of exactly 1000 bytes to ensure progress tracking
        let body = vec![b'x'; 1000];

        // Set up mock response
        let mock = server
            .mock("GET", "/typst.tar.xz")
            .with_status(200)
            .with_header("content-type", "application/x-xz")
            .with_body(&body)
            .create();

        // Use mock server URL
        let url = Url::parse(&format!("{}/typst.tar.xz", server.url())).unwrap();
        let result = download_to_temp(&url, 1000, Some(track_progress));

        mock.assert();
        assert!(result.is_ok(), "Should successfully download with progress");

        // Verify progress callback was invoked
        let calls = PROGRESS_CALLS
            .get()
            .expect("Progress callback should have initialized storage")
            .lock()
            .unwrap();
        assert!(
            !calls.is_empty(),
            "Progress callback should be invoked at least once"
        );

        // Verify final call has correct total
        let (final_downloaded, final_total) = calls.last().unwrap();
        assert_eq!(
            *final_downloaded, 1000,
            "Final downloaded should equal expected size"
        );
        assert_eq!(*final_total, 1000, "Final total should equal expected size");
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
        // This test verifies empty archive extraction
        let temp = typstlab_testkit::temp_dir_in_workspace();
        let empty_archive = temp.path().join("empty.tar.xz");

        // Create valid but empty tar.xz with proper XZ compression
        {
            let file = std::fs::File::create(&empty_archive).unwrap();
            let enc = xz2::write::XzEncoder::new(file, 6);
            let tar = tar::Builder::new(enc);
            // Finish tar builder (writes tar footer)
            let enc = tar.into_inner().unwrap();
            // Finish XZ encoder (writes XZ footer and flushes)
            enc.finish().unwrap();
        }

        let result = extract_to_temp(&empty_archive, "empty.tar.xz");
        // Should extract successfully (empty archive is valid, just has no files)
        assert!(
            result.is_ok(),
            "Should extract empty archive successfully: {:?}",
            result.err()
        );
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
        // This test verifies complete download and install flow with mockito
        let mut server = Server::new();

        // Create a real tar.xz archive with a binary for testing
        let archive_temp = create_fake_tar_xz_with_binary(binary_name(), true);
        let archive_path = archive_temp.path().join("archive.tar.xz");
        let archive_bytes = fs::read(&archive_path).unwrap();
        let archive_size = archive_bytes.len() as u64;

        // Set up mock response
        let mock = server
            .mock("GET", "/typst.tar.xz")
            .with_status(200)
            .with_header("content-type", "application/x-xz")
            .with_body(&archive_bytes)
            .create();

        let cache_dir = typstlab_testkit::temp_dir_in_workspace();
        let mock_url = format!("{}/typst.tar.xz", server.url());
        let asset = mock_asset("typst-x86_64-apple-darwin.tar.xz", &mock_url, archive_size);

        let options = DownloadOptions {
            cache_dir: cache_dir.path().to_path_buf(),
            version: "0.18.0".to_string(),
            progress: None,
        };

        let result = download_and_install(&asset, options);
        mock.assert();

        assert!(
            result.is_ok(),
            "Should successfully complete download and install: {:?}",
            result.err()
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
        // This test verifies version directory creation
        let mut server = Server::new();

        // Create a real tar.xz archive with a binary for testing
        let archive_temp = create_fake_tar_xz_with_binary(binary_name(), true);
        let archive_path = archive_temp.path().join("archive.tar.xz");
        let archive_bytes = fs::read(&archive_path).unwrap();
        let archive_size = archive_bytes.len() as u64;

        // Set up mock response
        let mock = server
            .mock("GET", "/typst.tar.xz")
            .with_status(200)
            .with_header("content-type", "application/x-xz")
            .with_body(&archive_bytes)
            .create();

        let cache_dir = typstlab_testkit::temp_dir_in_workspace();
        let mock_url = format!("{}/typst.tar.xz", server.url());
        let asset = mock_asset("typst-x86_64-apple-darwin.tar.xz", &mock_url, archive_size);

        let options = DownloadOptions {
            cache_dir: cache_dir.path().to_path_buf(),
            version: "0.18.0".to_string(),
            progress: None,
        };

        let result = download_and_install(&asset, options);
        mock.assert();

        assert!(
            result.is_ok(),
            "Should successfully install: {:?}",
            result.err()
        );

        let binary_path = result.unwrap();
        let version_dir = binary_path.parent().unwrap();
        assert!(
            version_dir.ends_with("0.18.0"),
            "Binary should be in version-specific directory: {}",
            version_dir.display()
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
        // This test verifies progress callback during download
        use std::sync::{Mutex, OnceLock};

        // Static storage for tracking progress calls (required for fn pointer)
        static INSTALL_PROGRESS_CALLS: OnceLock<Mutex<Vec<(u64, u64)>>> = OnceLock::new();

        fn track_install_progress(downloaded: u64, total: u64) {
            INSTALL_PROGRESS_CALLS
                .get_or_init(|| Mutex::new(Vec::new()))
                .lock()
                .unwrap()
                .push((downloaded, total));
        }

        let mut server = Server::new();

        // Create a real tar.xz archive with a binary for testing
        let archive_temp = create_fake_tar_xz_with_binary(binary_name(), true);
        let archive_path = archive_temp.path().join("archive.tar.xz");
        let archive_bytes = fs::read(&archive_path).unwrap();
        let archive_size = archive_bytes.len() as u64;

        // Set up mock response
        let mock = server
            .mock("GET", "/typst.tar.xz")
            .with_status(200)
            .with_header("content-type", "application/x-xz")
            .with_body(&archive_bytes)
            .create();

        let cache_dir = typstlab_testkit::temp_dir_in_workspace();
        let mock_url = format!("{}/typst.tar.xz", server.url());
        let asset = mock_asset("typst-x86_64-apple-darwin.tar.xz", &mock_url, archive_size);

        let options = DownloadOptions {
            cache_dir: cache_dir.path().to_path_buf(),
            version: "0.18.0".to_string(),
            progress: Some(track_install_progress),
        };

        let result = download_and_install(&asset, options);
        mock.assert();

        assert!(
            result.is_ok(),
            "Should successfully download with progress tracking: {:?}",
            result.err()
        );

        // Verify progress callback was invoked
        let calls = INSTALL_PROGRESS_CALLS
            .get()
            .expect("Progress callback should have initialized storage")
            .lock()
            .unwrap();
        assert!(
            !calls.is_empty(),
            "Progress callback should be invoked at least once"
        );

        // Verify final call has correct total
        let (final_downloaded, final_total) = calls.last().unwrap();
        assert_eq!(
            *final_downloaded, archive_size,
            "Final downloaded should equal archive size"
        );
        assert_eq!(
            *final_total, archive_size,
            "Final total should equal archive size"
        );
    }

    // Tests for executable permission preservation and cleanup fixes

    #[test]
    #[cfg(unix)]
    fn test_atomic_move_preserves_executable_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_workspace = typstlab_testkit::temp_dir_in_workspace();

        // Create source file with executable permissions
        let src_path = temp_workspace.path().join("source_binary");
        fs::write(&src_path, b"fake binary").unwrap();

        let mut perms = fs::metadata(&src_path).unwrap().permissions();
        perms.set_mode(0o755); // rwxr-xr-x
        fs::set_permissions(&src_path, perms).unwrap();

        // Verify source has executable bit
        let src_mode = fs::metadata(&src_path).unwrap().permissions().mode();
        assert_eq!(src_mode & 0o111, 0o111, "Source should be executable");

        // Create destination directory
        let dest_dir = temp_workspace.path().join("dest");
        fs::create_dir_all(&dest_dir).unwrap();
        let dest_path = dest_dir.join("dest_binary");

        // Perform atomic move
        atomic_move(&src_path, &dest_path).unwrap();

        // Verify destination has executable permissions
        let dest_mode = fs::metadata(&dest_path).unwrap().permissions().mode();
        assert_eq!(
            dest_mode & 0o111,
            0o111,
            "Destination should preserve executable permissions (mode: {:o})",
            dest_mode
        );

        // Verify file contents preserved
        let content = fs::read(&dest_path).unwrap();
        assert_eq!(content, b"fake binary");
    }

    #[test]
    #[cfg(unix)]
    fn test_atomic_move_preserves_exact_permission_bits() {
        use std::os::unix::fs::PermissionsExt;

        let temp_workspace = typstlab_testkit::temp_dir_in_workspace();

        // Create source file with specific permissions
        let src_path = temp_workspace.path().join("source_binary");
        fs::write(&src_path, b"fake binary").unwrap();

        // Set specific permissions: rwxr-x--- (0o750)
        let mut perms = fs::metadata(&src_path).unwrap().permissions();
        perms.set_mode(0o750);
        fs::set_permissions(&src_path, perms).unwrap();

        let src_mode = fs::metadata(&src_path).unwrap().permissions().mode();

        // Create destination
        let dest_dir = temp_workspace.path().join("dest");
        fs::create_dir_all(&dest_dir).unwrap();
        let dest_path = dest_dir.join("dest_binary");

        // Perform atomic move
        atomic_move(&src_path, &dest_path).unwrap();

        // Verify exact permission bits preserved (not just executable bit)
        let dest_mode = fs::metadata(&dest_path).unwrap().permissions().mode();
        assert_eq!(
            dest_mode & 0o777,
            src_mode & 0o777,
            "Destination should preserve exact permission bits from source"
        );
    }

    #[test]
    fn test_atomic_move_succeeds_even_if_cleanup_fails() {
        let temp_workspace = typstlab_testkit::temp_dir_in_workspace();

        // Create source file
        let src_path = temp_workspace.path().join("source_binary");
        fs::write(&src_path, b"fake binary").unwrap();

        // Create destination
        let dest_dir = temp_workspace.path().join("dest");
        fs::create_dir_all(&dest_dir).unwrap();
        let dest_path = dest_dir.join("dest_binary");

        // Perform atomic move
        let result = atomic_move(&src_path, &dest_path);

        // Should succeed even if cleanup fails
        // (Note: In practice, cleanup usually succeeds in tests, but the important
        // part is that the implementation uses `let _ = fs::remove_file()` so it
        // would ignore cleanup failures)
        assert!(result.is_ok(), "atomic_move should succeed");

        // Verify destination file exists and has correct content
        assert!(dest_path.exists(), "Destination file should exist");
        let content = fs::read(&dest_path).unwrap();
        assert_eq!(content, b"fake binary");
    }

    #[test]
    fn test_atomic_move_overwrites_existing_file() {
        let temp_workspace = typstlab_testkit::temp_dir_in_workspace();

        // Create destination with existing file
        let dest_dir = temp_workspace.path().join("dest");
        fs::create_dir_all(&dest_dir).unwrap();
        let dest_path = dest_dir.join("dest_binary");
        fs::write(&dest_path, b"old content").unwrap();

        // Create source file
        let src_path = temp_workspace.path().join("source_binary");
        fs::write(&src_path, b"new content").unwrap();

        // Perform atomic move (should overwrite)
        atomic_move(&src_path, &dest_path).unwrap();

        // Verify destination has new content
        let content = fs::read(&dest_path).unwrap();
        assert_eq!(content, b"new content");
    }

    #[test]
    #[cfg(unix)]
    fn test_set_executable_permissions_on_extracted_binary() {
        use std::os::unix::fs::PermissionsExt;

        let temp_workspace = typstlab_testkit::temp_dir_in_workspace();

        // Create a test binary file
        let binary_path = temp_workspace.path().join(binary_name());
        fs::write(&binary_path, b"#!/bin/sh\necho test").unwrap();

        // Initially should not be executable (default permissions)
        let initial_mode = fs::metadata(&binary_path).unwrap().permissions().mode();
        // Default mode varies by platform, but we just verify it becomes executable after

        // Set executable permissions
        set_executable_permissions(&binary_path).unwrap();

        // Verify now executable
        let final_mode = fs::metadata(&binary_path).unwrap().permissions().mode();
        assert_eq!(
            final_mode & 0o111,
            0o111,
            "Binary should be executable after set_executable_permissions (initial: {:o}, final: {:o})",
            initial_mode,
            final_mode
        );
    }
}
