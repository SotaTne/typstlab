//! Fixture management for testing
//!
//! This module provides utilities for setting up test fixtures,
//! particularly for extracting pre-downloaded typst binaries from
//! the fixtures directory to avoid network access during tests.

use std::path::{Path, PathBuf};

/// Get platform-specific archive name for typst
fn get_archive_name() -> &'static str {
    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    {
        "typst-x86_64-apple-darwin.tar.xz"
    }

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        "typst-aarch64-apple-darwin.tar.xz"
    }

    #[cfg(all(target_arch = "x86_64", target_os = "windows"))]
    {
        "typst-x86_64-pc-windows-msvc.zip"
    }

    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    {
        "typst-x86_64-unknown-linux-musl.tar.xz"
    }

    #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
    {
        "typst-aarch64-unknown-linux-musl.tar.xz"
    }

    #[cfg(not(any(
        all(target_arch = "x86_64", target_os = "macos"),
        all(target_arch = "aarch64", target_os = "macos"),
        all(target_arch = "x86_64", target_os = "windows"),
        all(target_arch = "x86_64", target_os = "linux"),
        all(target_arch = "aarch64", target_os = "linux")
    )))]
    {
        compile_error!(
            "Unsupported platform for typst fixtures. \
             Supported platforms: \
             x86_64-apple-darwin, aarch64-apple-darwin, \
             x86_64-pc-windows-msvc, \
             x86_64-unknown-linux-musl, aarch64-unknown-linux-musl"
        );
        ""
    }
}

/// Extract typst binary from fixtures to cache directory
///
/// This helper extracts the pre-downloaded typst binary from fixtures
/// to the isolated cache directory, avoiding GitHub API calls.
///
/// # Arguments
///
/// * `cache_dir` - Path to isolated cache directory (from TYPSTLAB_CACHE_DIR)
///
/// # Returns
///
/// Ok(PathBuf) to extracted binary, or Err(String) if extraction fails
///
/// # Errors
///
/// Returns Err if:
/// - Unable to determine platform
/// - Fixtures archive not found
/// - Archive extraction fails
/// - Binary not found in archive
pub fn setup_typst_from_fixtures(cache_dir: &Path) -> Result<PathBuf, String> {
    let archive_name = get_archive_name();

    // Path to fixtures in project root
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| "CARGO_MANIFEST_DIR not set".to_string())?;
    let fixtures_path = PathBuf::from(manifest_dir)
        .parent() // crates/typstlab-testkit -> crates
        .ok_or("Failed to get crates directory")?
        .parent() // crates -> project root
        .ok_or("Failed to get project root")?
        .join("fixtures")
        .join("typst")
        .join("v0.12.0")
        .join(archive_name);

    // Read archive
    let archive_bytes = std::fs::read(&fixtures_path)
        .map_err(|e| format!("Failed to read {} from fixtures: {}", archive_name, e))?;

    // Create version directory
    let version_dir = cache_dir.join("0.12.0");
    std::fs::create_dir_all(&version_dir)
        .map_err(|e| format!("Failed to create version dir: {}", e))?;

    // Extract binary based on platform
    extract_binary_from_archive(&archive_bytes, &version_dir, archive_name)
}

/// Extract binary from tar.xz archive (Unix)
#[cfg(not(target_os = "windows"))]
fn extract_from_tar_xz(
    archive_bytes: &[u8],
    version_dir: &Path,
    archive_name: &str,
) -> Result<PathBuf, String> {
    use tar::Archive;
    use xz2::read::XzDecoder;

    let decoder = XzDecoder::new(archive_bytes);
    let mut archive = Archive::new(decoder);

    let entries = archive
        .entries()
        .map_err(|e| format!("Failed to read archive entries: {}", e))?;

    for entry_result in entries {
        let mut entry = entry_result.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry
            .path()
            .map_err(|e| format!("Failed to get entry path: {}", e))?;

        // Find binary (typst-{arch}-{os}/typst)
        if path.file_name().map(|n| n == "typst").unwrap_or(false) {
            let binary_path = version_dir.join("typst");
            let mut output = std::fs::File::create(&binary_path)
                .map_err(|e| format!("Failed to create binary file: {}", e))?;
            std::io::copy(&mut entry, &mut output)
                .map_err(|e| format!("Failed to extract binary: {}", e))?;

            // Make executable on Unix
            make_executable(&binary_path)?;

            return Ok(binary_path);
        }
    }

    Err(format!("Failed to find typst binary in {}", archive_name))
}

/// Make file executable (Unix only)
#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)
        .map_err(|e| format!("Failed to get metadata: {}", e))?
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).map_err(|e| format!("Failed to set permissions: {}", e))
}

/// Extract binary from zip archive (Windows)
#[cfg(target_os = "windows")]
fn extract_from_zip(
    archive_bytes: &[u8],
    version_dir: &Path,
    archive_name: &str,
) -> Result<PathBuf, String> {
    use std::io::Cursor;
    use zip::ZipArchive;

    let reader = Cursor::new(archive_bytes);
    let mut archive = ZipArchive::new(reader).map_err(|e| format!("Failed to read zip: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to get zip entry: {}", e))?;
        if file.name().ends_with("typst.exe") {
            let binary_path = version_dir.join("typst.exe");
            let mut output = std::fs::File::create(&binary_path)
                .map_err(|e| format!("Failed to create binary file: {}", e))?;
            std::io::copy(&mut file, &mut output)
                .map_err(|e| format!("Failed to extract binary: {}", e))?;
            return Ok(binary_path);
        }
    }

    Err(format!("Failed to find typst.exe in {}", archive_name))
}

/// Extract binary from archive (tar.xz or zip)
fn extract_binary_from_archive(
    archive_bytes: &[u8],
    version_dir: &Path,
    archive_name: &str,
) -> Result<PathBuf, String> {
    #[cfg(not(target_os = "windows"))]
    {
        extract_from_tar_xz(archive_bytes, version_dir, archive_name)
    }

    #[cfg(target_os = "windows")]
    {
        extract_from_zip(archive_bytes, version_dir, archive_name)
    }
}

/// Setup test typst binary in isolated environment
///
/// This helper extracts typst 0.12.0 from fixtures to the isolated cache directory
/// and returns the path to the binary. No network access required.
///
/// # Arguments
///
/// * `typstlab_bin` - Path to the typstlab binary (unused, kept for API compatibility)
/// * `project_dir` - Project directory (unused, kept for API compatibility)
///
/// # Returns
///
/// PathBuf to the extracted typst binary
///
/// # Panics
///
/// Panics if:
/// - TYPSTLAB_CACHE_DIR environment variable not set
/// - Fixtures archive not found
/// - Binary extraction fails
///
/// # Examples
///
/// ```no_run
/// use typstlab_testkit::{with_isolated_typst_env, setup_test_typst};
/// use assert_cmd::cargo::CommandCargoExt;
/// use std::path::PathBuf;
/// use std::process::Command;
///
/// // Example test function
/// fn test_with_typst() {
///     with_isolated_typst_env(None, |_cache| {
///         let temp = tempfile::TempDir::new().unwrap();
///         let project_dir = temp.path();
///
///         // Get typstlab binary path
///         let typstlab = PathBuf::from(Command::cargo_bin("typstlab").unwrap().get_program());
///
///         // Extract typst from fixtures (no network access)
///         let typst_path = setup_test_typst(&typstlab, project_dir);
///
///         // Now use typst_path in your test
///     });
/// }
/// ```
pub fn setup_test_typst(_typstlab_bin: &Path, _project_dir: &Path) -> PathBuf {
    // Get cache directory from environment (set by with_isolated_typst_env)
    let cache_dir = std::env::var("TYPSTLAB_CACHE_DIR")
        .expect("TYPSTLAB_CACHE_DIR should be set by with_isolated_typst_env");
    let cache_path = PathBuf::from(cache_dir);

    // Extract binary from fixtures to cache directory (no GitHub API)
    setup_typst_from_fixtures(&cache_path)
        .expect("Failed to setup typst from fixtures - ensure fixtures/typst/v0.12.0/ exists")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::with_isolated_typst_env;
    use crate::temp::temp_dir_in_workspace;

    #[test]
    fn test_setup_test_typst_installs_binary() {
        with_isolated_typst_env(None, |_cache| {
            let temp = temp_dir_in_workspace();
            let project_dir = temp.path();

            // Create a minimal project structure with typstlab.toml
            std::fs::create_dir_all(project_dir.join(".typstlab")).unwrap();
            std::fs::write(
                project_dir.join("typstlab.toml"),
                r#"
[project]
name = "test"
init_date = "2026-01-15"
[typst]
version = "0.12.0"
"#,
            )
            .unwrap();

            // Get typstlab binary path
            use assert_cmd::cargo::CommandCargoExt;
            use std::process::Command;
            #[allow(deprecated)]
            let typstlab_bin =
                std::path::PathBuf::from(Command::cargo_bin("typstlab").unwrap().get_program());

            // Setup typst
            let typst_path = setup_test_typst(&typstlab_bin, project_dir);

            // Verify binary exists
            assert!(
                typst_path.exists(),
                "Typst binary should exist at: {}",
                typst_path.display()
            );

            // Verify it's in the cache directory
            let cache_dir = std::env::var("TYPSTLAB_CACHE_DIR").unwrap();
            assert!(
                typst_path.to_string_lossy().contains(&cache_dir),
                "Typst binary should be in cache directory"
            );

            // Verify version directory
            assert!(
                typst_path.to_string_lossy().contains("0.12.0"),
                "Typst binary should be in version directory"
            );
        });
    }
}
