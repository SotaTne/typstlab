//! Test utilities for typstlab
//!
//! This crate provides shared testing utilities used across the typstlab workspace.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tempfile::TempDir;

/// Static mutex to serialize tests that modify environment variables
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Creates a temporary directory within `.tmp/` at the project root
///
/// This ensures all test temporary files are centralized in a single location
/// that is gitignored and easy to clean up manually if needed.
///
/// # Returns
///
/// A `TempDir` instance that automatically cleans up on drop.
/// The directory is created at `.tmp/<random-name>` relative to the project root.
///
/// # Panics
///
/// Panics if:
/// - Unable to determine current directory
/// - Unable to create `.tmp/` directory
/// - Unable to create temporary subdirectory
///
/// # Examples
///
/// ```rust
/// use typstlab_testkit::temp_dir_in_workspace;
///
/// let temp = temp_dir_in_workspace();
/// let file_path = temp.path().join("test.txt");
/// std::fs::write(&file_path, "test data").unwrap();
/// // Cleanup happens automatically when temp is dropped
/// ```
pub fn temp_dir_in_workspace() -> TempDir {
    let workspace_root = std::env::current_dir().expect("Failed to get current directory");

    let tmp_base = workspace_root.join(".tmp");

    // Ensure .tmp/ exists
    std::fs::create_dir_all(&tmp_base).expect("Failed to create .tmp directory");

    // Create unique subdirectory within .tmp/
    TempDir::new_in(&tmp_base).expect("Failed to create temporary directory in .tmp/")
}

/// Alternative with Result for non-test code
///
/// Use this variant when you need proper error handling instead of panics.
pub fn try_temp_dir_in_workspace() -> std::io::Result<TempDir> {
    let workspace_root = std::env::current_dir()?;
    let tmp_base = workspace_root.join(".tmp");
    std::fs::create_dir_all(&tmp_base)?;
    TempDir::new_in(&tmp_base)
}

/// Run a test with isolated typst environment
///
/// This helper provides complete environment isolation for tests that need to control
/// typst binary resolution. It:
/// 1. Creates an isolated HOME directory
/// 2. Creates an isolated cache directory
/// 3. Controls TYPST_BINARY environment variable
/// 4. Prevents tests from interfering with each other using a Mutex
///
/// # Arguments
///
/// * `typst_binary` - Optional path to a typst binary. If `None`, no typst will be available.
/// * `f` - Test closure that receives the cache directory path
///
/// # Returns
///
/// The result returned by the test closure
///
/// # Examples
///
/// ## Test with typst NOT found
///
/// ```no_run
/// use typstlab_testkit::with_isolated_typst_env;
/// use std::process::Command;
///
/// // Example test function (not executed in doctest)
/// fn test_typst_not_found() {
///     with_isolated_typst_env(None, |_cache| {
///         // In this test, typst is NOT available
///         // Neither in cache, nor via TYPST_BINARY, nor in PATH
///         let result = Command::new("typstlab")
///             .arg("sync")
///             .status();
///         // Should fail because typst not found
///     });
/// }
/// ```
///
/// ## Test with specific typst binary
///
/// ```no_run
/// use typstlab_testkit::with_isolated_typst_env;
/// use std::path::PathBuf;
///
/// // Example test function (not executed in doctest)
/// fn test_with_specific_typst() {
///     let typst_path = PathBuf::from("/usr/local/bin/typst");
///     with_isolated_typst_env(Some(&typst_path), |_cache| {
///         // In this test, TYPST_BINARY points to /usr/local/bin/typst
///         // typstlab will use this binary
///     });
/// }
/// ```
pub fn with_isolated_typst_env<F, R>(typst_binary: Option<&Path>, f: F) -> R
where
    F: FnOnce(&Path) -> R,
{
    let _guard = ENV_LOCK.lock().unwrap();

    // Save original environment (for restoration)
    let original_home = std::env::var("HOME").ok();
    let original_cache_dir = std::env::var("TYPSTLAB_CACHE_DIR").ok();
    let original_typst_binary = std::env::var("TYPST_BINARY").ok();

    // Create isolated directories
    let fake_home = TempDir::new().unwrap();
    let fake_cache = fake_home.path().join(".cache/typstlab");
    std::fs::create_dir_all(&fake_cache).unwrap();

    // Set environment variables for COMPLETE isolation
    // SAFETY: We hold ENV_LOCK, ensuring no other test is modifying env vars concurrently.
    // Environment variable modification is inherently unsafe in multi-threaded contexts,
    // but the mutex guarantees exclusive access, making this safe.
    unsafe {
        std::env::set_var("HOME", fake_home.path());
        std::env::set_var("TYPSTLAB_CACHE_DIR", &fake_cache);

        if let Some(binary_path) = typst_binary {
            std::env::set_var("TYPST_BINARY", binary_path);
        } else {
            // Ensure TYPST_BINARY is not set (test "not found" scenario)
            std::env::remove_var("TYPST_BINARY");
        }
    }

    // Run test
    let result = f(fake_cache.as_path());

    // Restore environment (important for test isolation)
    // SAFETY: We still hold ENV_LOCK, ensuring exclusive access to env vars.
    unsafe {
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        if let Some(cache_dir) = original_cache_dir {
            std::env::set_var("TYPSTLAB_CACHE_DIR", cache_dir);
        } else {
            std::env::remove_var("TYPSTLAB_CACHE_DIR");
        }

        if let Some(binary) = original_typst_binary {
            std::env::set_var("TYPST_BINARY", binary);
        } else {
            std::env::remove_var("TYPST_BINARY");
        }
    }

    result
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
/// Path to extracted binary
///
/// # Panics
///
/// Panics if:
/// - Unable to determine platform
/// - Fixtures archive not found
/// - Archive extraction fails
/// - Binary not found in archive
fn setup_typst_from_fixtures(cache_dir: &Path) -> PathBuf {
    // Determine platform-specific archive name
    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    let archive_name = "typst-x86_64-apple-darwin.tar.xz";

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    let archive_name = "typst-aarch64-apple-darwin.tar.xz";

    #[cfg(all(target_arch = "x86_64", target_os = "windows"))]
    let archive_name = "typst-x86_64-pc-windows-msvc.zip";

    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    let archive_name = "typst-x86_64-unknown-linux-musl.tar.xz";

    #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
    let archive_name = "typst-aarch64-unknown-linux-musl.tar.xz";

    // Path to fixtures in project root
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set");
    let fixtures_path = PathBuf::from(manifest_dir)
        .parent() // crates/typstlab-testkit -> crates
        .expect("Failed to get crates directory")
        .parent() // crates -> project root
        .expect("Failed to get project root")
        .join("fixtures")
        .join("typst")
        .join("v0.12.0")
        .join(archive_name);

    // Read archive
    let archive_bytes = std::fs::read(&fixtures_path)
        .unwrap_or_else(|e| panic!("Failed to read {} from fixtures: {}", archive_name, e));

    // Create version directory
    let version_dir = cache_dir.join("0.12.0");
    std::fs::create_dir_all(&version_dir).expect("Failed to create version dir");

    // Extract binary based on platform
    extract_binary_from_archive(&archive_bytes, &version_dir, archive_name)
}

/// Extract binary from archive (tar.xz or zip)
fn extract_binary_from_archive(
    archive_bytes: &[u8],
    version_dir: &Path,
    archive_name: &str,
) -> PathBuf {
    // Handle tar.xz archives (Unix)
    #[cfg(not(target_os = "windows"))]
    {
        use tar::Archive;
        use xz2::read::XzDecoder;

        let decoder = XzDecoder::new(archive_bytes);
        let mut archive = Archive::new(decoder);

        for entry in archive.entries().expect("Failed to read archive entries") {
            let mut entry = entry.expect("Failed to read entry");
            let path = entry.path().expect("Failed to get entry path");

            // Find binary (typst-{arch}-{os}/typst)
            if path.file_name().map(|n| n == "typst").unwrap_or(false) {
                let binary_path = version_dir.join("typst");
                let mut output =
                    std::fs::File::create(&binary_path).expect("Failed to create binary file");
                std::io::copy(&mut entry, &mut output).expect("Failed to extract binary");

                // Make executable
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = std::fs::metadata(&binary_path)
                        .expect("Failed to get metadata")
                        .permissions();
                    perms.set_mode(0o755);
                    std::fs::set_permissions(&binary_path, perms)
                        .expect("Failed to set permissions");
                }

                return binary_path;
            }
        }

        panic!("Failed to find typst binary in {}", archive_name);
    }

    // Handle zip archives (Windows)
    #[cfg(target_os = "windows")]
    {
        use std::io::Cursor;
        use zip::ZipArchive;

        let reader = Cursor::new(archive_bytes);
        let mut archive = ZipArchive::new(reader).expect("Failed to read zip");

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).expect("Failed to get zip entry");
            if file.name().ends_with("typst.exe") {
                let binary_path = version_dir.join("typst.exe");
                let mut output =
                    std::fs::File::create(&binary_path).expect("Failed to create binary file");
                std::io::copy(&mut file, &mut output).expect("Failed to extract binary");
                return binary_path;
            }
        }

        panic!("Failed to find typst.exe in {}", archive_name);
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
}

/// Get the path to a compiled example binary
///
/// This helper locates example binaries compiled by cargo test.
/// Example binaries are in the `target/debug/examples/` directory.
///
/// # Arguments
///
/// * `name` - Name of the example binary (without .exe extension)
///
/// # Returns
///
/// PathBuf to the compiled example binary
///
/// # Panics
///
/// Panics if unable to determine the current executable path
///
/// # Examples
///
/// ```no_run
/// use typstlab_testkit::example_bin;
/// use std::process::Command;
///
/// // Example test function (not executed in doctest)
/// fn test_with_example() {
///     let status = Command::new(example_bin("counter_child"))
///         .arg("counter.txt")
///         .arg("10")
///         .status()
///         .unwrap();
///     assert!(status.success());
/// }
/// ```
pub fn example_bin(name: &str) -> PathBuf {
    let mut path = std::env::current_exe().expect("Failed to get current executable path");

    // Navigate from target/debug/deps/test_binary to target/debug/examples/
    path.pop(); // Remove test binary name
    path.pop(); // Remove "deps"
    path.push("examples");
    path.push(name);

    // Add .exe extension on Windows
    if cfg!(target_os = "windows") {
        path.set_extension("exe");
    }

    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temp_dir_in_workspace_creates_in_tmp() {
        let temp = temp_dir_in_workspace();
        let path = temp.path();

        // Verify path contains .tmp
        assert!(
            path.to_string_lossy().contains(".tmp"),
            "Path should contain .tmp, got: {}",
            path.display()
        );

        // Verify directory exists
        assert!(path.exists(), "Directory should exist");
        assert!(path.is_dir(), "Path should be a directory");
    }

    #[test]
    fn test_temp_dir_auto_cleanup() {
        let path = {
            let temp = temp_dir_in_workspace();
            let p = temp.path().to_path_buf();
            assert!(p.exists(), "Directory should exist before drop");
            p
        }; // temp dropped here

        // Directory should be cleaned up
        assert!(
            !path.exists(),
            "Directory should not exist after drop: {}",
            path.display()
        );
    }

    #[test]
    fn test_multiple_temp_dirs_unique() {
        let temp1 = temp_dir_in_workspace();
        let temp2 = temp_dir_in_workspace();

        // Should have different paths
        assert_ne!(
            temp1.path(),
            temp2.path(),
            "Multiple temp directories should have unique paths"
        );
    }

    #[test]
    fn test_try_temp_dir_in_workspace_returns_ok() {
        let result = try_temp_dir_in_workspace();
        assert!(result.is_ok(), "Should successfully create temp directory");

        let temp = result.unwrap();
        assert!(temp.path().exists());
        assert!(temp.path().to_string_lossy().contains(".tmp"));
    }

    #[test]
    fn test_with_isolated_typst_env_creates_fake_home() {
        with_isolated_typst_env(None, |cache_dir| {
            // Verify cache directory exists
            assert!(cache_dir.exists(), "Cache directory should exist");
            assert!(
                cache_dir.to_string_lossy().contains(".cache"),
                "Cache should be in .cache directory"
            );

            // Verify HOME environment variable is set to fake home
            let home = std::env::var("HOME").unwrap();
            assert!(
                home.contains("tmp"),
                "HOME should point to temporary directory"
            );

            // Verify TYPSTLAB_CACHE_DIR is set
            let cache_env = std::env::var("TYPSTLAB_CACHE_DIR").unwrap();
            assert_eq!(
                cache_env,
                cache_dir.to_string_lossy(),
                "TYPSTLAB_CACHE_DIR should match provided cache_dir"
            );

            // Verify TYPST_BINARY is not set (since we passed None)
            assert!(
                std::env::var("TYPST_BINARY").is_err(),
                "TYPST_BINARY should not be set when None is passed"
            );
        });
    }

    #[test]
    fn test_with_isolated_typst_env_sets_typst_binary() {
        let fake_binary = PathBuf::from("/fake/typst");

        with_isolated_typst_env(Some(&fake_binary), |_cache| {
            // Verify TYPST_BINARY is set to the fake binary
            let binary_env = std::env::var("TYPST_BINARY").unwrap();
            assert_eq!(
                binary_env,
                fake_binary.to_string_lossy(),
                "TYPST_BINARY should be set to provided path"
            );
        });
    }

    #[test]
    fn test_with_isolated_typst_env_restores_original_env() {
        // Save original environment
        let original_home = std::env::var("HOME").ok();
        let original_cache = std::env::var("TYPSTLAB_CACHE_DIR").ok();
        let original_binary = std::env::var("TYPST_BINARY").ok();

        // Run isolated environment
        with_isolated_typst_env(None, |_cache| {
            // Environment is modified inside
        });

        // Verify environment is restored
        assert_eq!(
            std::env::var("HOME").ok(),
            original_home,
            "HOME should be restored"
        );
        assert_eq!(
            std::env::var("TYPSTLAB_CACHE_DIR").ok(),
            original_cache,
            "TYPSTLAB_CACHE_DIR should be restored"
        );
        assert_eq!(
            std::env::var("TYPST_BINARY").ok(),
            original_binary,
            "TYPST_BINARY should be restored"
        );
    }

    #[test]
    fn test_with_isolated_typst_env_serializes_access() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::thread;

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone1 = Arc::clone(&counter);
        let counter_clone2 = Arc::clone(&counter);

        let handle1 = thread::spawn(move || {
            with_isolated_typst_env(None, |_cache| {
                let current = counter_clone1.fetch_add(1, Ordering::SeqCst);
                // If serialization works, current should always be 0
                // because no other thread can be inside at the same time
                thread::sleep(std::time::Duration::from_millis(10));
                assert_eq!(
                    current, 0,
                    "Thread 1: Should be the only thread inside (serialization)"
                );
                counter_clone1.fetch_sub(1, Ordering::SeqCst);
            });
        });

        let handle2 = thread::spawn(move || {
            with_isolated_typst_env(None, |_cache| {
                let current = counter_clone2.fetch_add(1, Ordering::SeqCst);
                // If serialization works, current should always be 0
                thread::sleep(std::time::Duration::from_millis(10));
                assert_eq!(
                    current, 0,
                    "Thread 2: Should be the only thread inside (serialization)"
                );
                counter_clone2.fetch_sub(1, Ordering::SeqCst);
            });
        });

        handle1.join().unwrap();
        handle2.join().unwrap();

        // Final counter should be 0 (all increments/decrements balanced)
        assert_eq!(counter.load(Ordering::SeqCst), 0, "Counter should be 0");
    }

    #[test]
    fn test_example_bin_returns_correct_path() {
        let path = example_bin("test_example");

        // Verify path contains "examples" directory
        assert!(
            path.to_string_lossy().contains("examples"),
            "Path should contain 'examples' directory"
        );

        // Verify path ends with the binary name
        let file_name = path.file_name().unwrap().to_string_lossy();
        assert!(
            file_name.starts_with("test_example"),
            "File name should start with 'test_example'"
        );

        // Verify .exe extension on Windows
        #[cfg(target_os = "windows")]
        assert!(
            file_name.ends_with(".exe"),
            "File should have .exe extension on Windows"
        );

        // Verify no .exe extension on Unix
        #[cfg(not(target_os = "windows"))]
        assert!(
            !file_name.ends_with(".exe"),
            "File should not have .exe extension on Unix"
        );
    }

    #[test]
    #[allow(deprecated)] // cargo_bin is deprecated but cargo_bin! macro doesn't work in lib tests
    fn test_setup_test_typst_installs_binary() {
        with_isolated_typst_env(None, |_cache| {
            let temp = temp_dir_in_workspace();
            let project_dir = temp.path();

            // Create a minimal project structure with typstlab.toml
            std::fs::create_dir_all(project_dir.join(".typstlab")).unwrap();
            std::fs::write(
                project_dir.join("typstlab.toml"),
                "[project]\nname = \"test\"\ninit_date = \"2026-01-15\"\n\n[typst]\nversion = \"0.12.0\"\n",
            )
            .unwrap();

            // Get typstlab binary path
            use assert_cmd::cargo::CommandCargoExt;
            use std::process::Command;
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
