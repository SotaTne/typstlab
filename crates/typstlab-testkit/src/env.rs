//! Environment isolation utilities for testing
//!
//! This module provides functions for isolating environment variables
//! during tests to prevent interference between parallel test executions.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tempfile::TempDir;

/// Static mutex to serialize tests that modify environment variables
pub static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Test-only snapshot of environment variables for deterministic restoration tests
///
/// This struct is always compiled (not just #[cfg(test)]) to avoid conditional
/// compilation complexity in the core function, but is only used in test code.
#[derive(Debug, Clone, PartialEq, Eq)]
struct EnvSnapshot {
    home: Option<String>,
    cache: Option<String>,
    typst: Option<String>,
}

/// Internal core function for environment isolation with optional snapshot capture
///
/// This function implements the common logic for `with_isolated_typst_env` and
/// `with_isolated_typst_env_snapshot`. The snapshot is captured AFTER restoration
/// but BEFORE releasing the mutex, ensuring deterministic tests.
fn with_isolated_typst_env_core<F, R>(
    typst_binary: Option<&Path>,
    f: F,
    #[cfg(test)] capture_snapshot: bool,
) -> (R, Option<EnvSnapshot>)
where
    F: FnOnce(&PathBuf) -> R,
{
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    // Save original environment (for restoration)
    let original_home = std::env::var("HOME").ok();
    let original_cache_dir = std::env::var("TYPSTLAB_CACHE_DIR").ok();
    let original_typst_binary = std::env::var("TYPST_BINARY").ok();

    // Create isolated directories and convert to String immediately
    let fake_home = TempDir::new().unwrap();
    let fake_home_str = fake_home
        .path()
        .to_str()
        .expect("TempDir path is not valid UTF-8")
        .to_string();

    let fake_cache_path = fake_home.path().join(".cache/typstlab");
    std::fs::create_dir_all(&fake_cache_path).unwrap();
    let fake_cache_str = fake_cache_path
        .to_str()
        .expect("Cache path is not valid UTF-8")
        .to_string();

    // Set environment variables
    // SAFETY: We hold ENV_LOCK, ensuring no other test is modifying env vars concurrently.
    unsafe {
        std::env::set_var("HOME", &fake_home_str);
        std::env::set_var("TYPSTLAB_CACHE_DIR", &fake_cache_str);

        if let Some(binary_path) = typst_binary {
            std::env::set_var("TYPST_BINARY", binary_path);
        } else {
            std::env::remove_var("TYPST_BINARY");
        }
    }

    // Run test
    let fake_cache_for_test = PathBuf::from(&fake_cache_str);
    let result = f(&fake_cache_for_test);

    // Explicit cleanup BEFORE TempDir::drop()
    let _ = std::fs::remove_dir_all(&fake_cache_path);
    drop(fake_home);

    // Restore environment
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

    // Capture snapshot AFTER restoration, BEFORE mutex release
    #[cfg(test)]
    let snapshot = if capture_snapshot {
        Some(EnvSnapshot {
            home: std::env::var("HOME").ok(),
            cache: std::env::var("TYPSTLAB_CACHE_DIR").ok(),
            typst: std::env::var("TYPST_BINARY").ok(),
        })
    } else {
        None
    };

    #[cfg(not(test))]
    let snapshot = None;

    (result, snapshot)
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
    F: FnOnce(&PathBuf) -> R,
{
    #[cfg(test)]
    let (result, _snapshot) = with_isolated_typst_env_core(typst_binary, f, false);

    #[cfg(not(test))]
    let (result, _snapshot) = with_isolated_typst_env_core(typst_binary, f);

    result
}

/// Test-only helper that captures env snapshot after restoration (race-proof)
///
/// This helper is identical to `with_isolated_typst_env` but captures
/// environment variable state BEFORE releasing the mutex, ensuring the
/// restoration test is deterministic and cannot race with other tests.
#[cfg(test)]
fn with_isolated_typst_env_snapshot<F, R>(typst_binary: Option<&Path>, f: F) -> (R, EnvSnapshot)
where
    F: FnOnce(&PathBuf) -> R,
{
    let (result, snapshot) = with_isolated_typst_env_core(typst_binary, f, true);
    (
        result,
        snapshot.expect("snapshot should be captured when requested"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_with_isolated_typst_env_restores_original_env_deterministic() {
        // Save original environment before test
        let original_home = std::env::var("HOME").ok();
        let original_cache = std::env::var("TYPSTLAB_CACHE_DIR").ok();
        let original_typst = std::env::var("TYPST_BINARY").ok();

        // Run isolated environment with snapshot capture
        let (_result, snapshot) = with_isolated_typst_env_snapshot(None, |_cache| {
            // Intentionally mutate env vars inside to ensure restoration happens
            unsafe {
                std::env::set_var("HOME", "/tmp/fake-home");
                std::env::set_var("TYPSTLAB_CACHE_DIR", "/tmp/fake-cache");
                std::env::set_var("TYPST_BINARY", "/tmp/fake-typst");
            }
        });

        // Verify environment is restored (snapshot captured BEFORE mutex release)
        // This test is deterministic and cannot race with other tests
        assert_eq!(
            snapshot.home, original_home,
            "HOME should be restored to original value"
        );
        assert_eq!(
            snapshot.cache, original_cache,
            "TYPSTLAB_CACHE_DIR should be restored to original value"
        );
        assert_eq!(
            snapshot.typst, original_typst,
            "TYPST_BINARY should be restored to original value"
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

    // ============================================================================
    // RED Phase Tests for Phase 2.10: Cache Persistence Fix (Linux CI Fix)
    // ============================================================================
    //
    // These tests verify that cache isolation works correctly between tests,
    // which is necessary to fix the Linux CI mock failure where docs cache
    // persists between tests due to delayed TempDir cleanup on tmpfs.

    #[test]
    fn test_cache_isolation_between_tests() {
        // Test 1: Create cache with marker file
        with_isolated_typst_env(None, |cache| {
            let marker = cache.join("test_marker.txt");
            std::fs::write(&marker, "test1").unwrap();
            assert!(marker.exists(), "Marker should exist in Test 1");
        });

        // Test 2: Cache should be clean (no marker from Test 1)
        with_isolated_typst_env(None, |cache| {
            let marker = cache.join("test_marker.txt");
            assert!(
                !marker.exists(),
                "Marker from Test 1 should NOT exist in Test 2 (cache isolation failure on Linux tmpfs)"
            );
        });
    }
}
