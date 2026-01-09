//! End-to-end integration tests for typstlab-typst
//!
//! These tests verify the complete flow from binary resolution to command execution.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::{NamedTempFile, TempDir};
use typstlab_typst::{ExecOptions, ResolveOptions, ResolveResult};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Helper function to create a fake typst binary in a temp directory
///
/// Uses NamedTempFile::persist() for atomic file creation to avoid
/// race conditions like Linux "Text file busy" (ETXTBSY) errors.
fn create_fake_typst_in_temp(temp_dir: &TempDir, version: &str, script_content: &str) -> PathBuf {
    let version_dir = temp_dir.path().join(version);
    fs::create_dir_all(&version_dir).unwrap();

    #[cfg(unix)]
    let binary_path = version_dir.join("typst");
    #[cfg(windows)]
    let binary_path = version_dir.join("typst.bat");

    #[cfg(unix)]
    {
        let script = format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  echo \"typst {}\"\n  exit 0\nfi\n{}",
            version, script_content
        );

        let mut temp_file = NamedTempFile::new_in(&version_dir).unwrap();
        temp_file.write_all(script.as_bytes()).unwrap();

        let mut perms = temp_file.as_file().metadata().unwrap().permissions();
        perms.set_mode(0o755);
        temp_file.as_file().set_permissions(perms).unwrap();

        // Ensure filesystem sync before execution to prevent ETXTBSY
        temp_file.as_file().sync_all().unwrap();

        // Persist and explicitly drop to avoid ETXTBSY on fast exec
        let persisted = temp_file.persist(&binary_path).unwrap();
        drop(persisted);
        sync_parent_dir(&version_dir);
    }

    #[cfg(windows)]
    {
        let script = format!(
            "@echo off\nif \"%1\"==\"--version\" (\n  echo typst {}\n  exit /b 0\n)\n{}",
            version, script_content
        );

        let mut temp_file = NamedTempFile::new_in(&version_dir).unwrap();
        temp_file.write_all(script.as_bytes()).unwrap();

        // Ensure filesystem sync before execution to prevent race conditions
        temp_file.as_file().sync_all().unwrap();

        // Persist and explicitly drop to avoid race on fast exec
        let persisted = temp_file.persist(&binary_path).unwrap();
        drop(persisted);
    }

    binary_path
}

fn sync_parent_dir(dir: &std::path::Path) {
    #[cfg(unix)]
    {
        if let Ok(handle) = std::fs::File::open(dir) {
            let _ = handle.sync_all();
        }
    }
}

/// Test complete flow: resolve managed binary -> execute command
#[test]
fn test_e2e_resolve_and_exec_managed() {
    let temp_cache = TempDir::new_in(std::env::current_dir().unwrap()).unwrap();
    let version = "0.18.0";

    let script_content = r#"echo "Hello from Typst"
exit 0
"#;
    let binary_path = create_fake_typst_in_temp(&temp_cache, version, script_content);

    // Step 1: Resolve the binary
    let resolve_options = ResolveOptions {
        required_version: version.to_string(),
        project_root: PathBuf::from("."),
        force_refresh: false,
    };

    let resolve_result = typstlab_typst::resolve::resolve_typst_with_override(
        resolve_options,
        Some(temp_cache.path().to_path_buf()),
    )
    .unwrap();

    // Verify resolution succeeded
    match resolve_result {
        ResolveResult::Resolved(info) => {
            assert_eq!(info.version, version);
            assert_eq!(info.path, binary_path);
        }
        _ => panic!("Expected Resolved result"),
    }

    // Step 2: Execute a command with the resolved binary
    let exec_options = ExecOptions {
        project_root: PathBuf::from("."),
        args: vec!["compile".to_string(), "test.typ".to_string()],
        required_version: version.to_string(),
    };

    let exec_result = typstlab_typst::exec::exec_typst_with_override(
        exec_options,
        Some(temp_cache.path().to_path_buf()),
    )
    .unwrap();

    // Verify execution succeeded
    assert_eq!(exec_result.exit_code, 0);
    assert!(exec_result.stdout.contains("Hello from Typst"));
    // Duration can be 0ms on very fast systems/CI (u64 is always >= 0, so no assertion needed)
    let _ = exec_result.duration_ms;

    // TempDir automatically cleans up
}

/// Test error case: binary not found
#[test]
fn test_e2e_binary_not_found() {
    let temp_cache = TempDir::new_in(std::env::current_dir().unwrap()).unwrap();
    let version = "99.99.99"; // Non-existent version

    let exec_options = ExecOptions {
        project_root: PathBuf::from("."),
        args: vec!["compile".to_string(), "test.typ".to_string()],
        required_version: version.to_string(),
    };

    // Should fail because binary cannot be resolved
    let result = typstlab_typst::exec::exec_typst_with_override(
        exec_options,
        Some(temp_cache.path().to_path_buf()),
    );
    assert!(result.is_err());

    // TempDir automatically cleans up
}

/// Test error case: binary execution fails
#[test]
fn test_e2e_execution_failure() {
    let temp_cache = TempDir::new_in(std::env::current_dir().unwrap()).unwrap();
    let version = "0.19.0";

    let script_content = r#"echo "Error: File not found" >&2
exit 1
"#;
    create_fake_typst_in_temp(&temp_cache, version, script_content);

    let exec_options = ExecOptions {
        project_root: PathBuf::from("."),
        args: vec!["compile".to_string(), "nonexistent.typ".to_string()],
        required_version: version.to_string(),
    };

    let exec_result = typstlab_typst::exec::exec_typst_with_override(
        exec_options,
        Some(temp_cache.path().to_path_buf()),
    )
    .unwrap();

    // Verify error was captured
    assert_eq!(exec_result.exit_code, 1);
    assert!(exec_result.stderr.contains("Error: File not found"));

    // TempDir automatically cleans up
}

/// Test force_refresh bypasses cache
#[test]
fn test_e2e_force_refresh() {
    let temp_cache = TempDir::new_in(std::env::current_dir().unwrap()).unwrap();
    let version = "0.20.0";

    create_fake_typst_in_temp(&temp_cache, version, "exit 0");

    // First resolve without force_refresh
    let resolve_options = ResolveOptions {
        required_version: version.to_string(),
        project_root: PathBuf::from("."),
        force_refresh: false,
    };

    let result1 = typstlab_typst::resolve::resolve_typst_with_override(
        resolve_options,
        Some(temp_cache.path().to_path_buf()),
    )
    .unwrap();
    assert!(matches!(result1, ResolveResult::Resolved(_)));

    // Second resolve with force_refresh should still work
    let resolve_options = ResolveOptions {
        required_version: version.to_string(),
        project_root: PathBuf::from("."),
        force_refresh: true,
    };

    let result2 = typstlab_typst::resolve::resolve_typst_with_override(
        resolve_options,
        Some(temp_cache.path().to_path_buf()),
    )
    .unwrap();
    assert!(matches!(result2, ResolveResult::Resolved(_)));

    // TempDir automatically cleans up
}

/// Test execution with different exit codes
#[test]
fn test_e2e_various_exit_codes() {
    let temp_cache = TempDir::new_in(std::env::current_dir().unwrap()).unwrap();
    let version = "0.21.0";

    // Test exit code 42
    let script_content = "exit 42";
    create_fake_typst_in_temp(&temp_cache, version, script_content);

    let exec_options = ExecOptions {
        project_root: PathBuf::from("."),
        args: vec!["test".to_string()],
        required_version: version.to_string(),
    };

    let exec_result = typstlab_typst::exec::exec_typst_with_override(
        exec_options,
        Some(temp_cache.path().to_path_buf()),
    )
    .unwrap();
    assert_eq!(exec_result.exit_code, 42);

    // TempDir automatically cleans up
}

/// Test stdout and stderr are properly captured
#[test]
fn test_e2e_output_capture() {
    let temp_cache = TempDir::new_in(std::env::current_dir().unwrap()).unwrap();
    let version = "0.22.0";

    #[cfg(unix)]
    let script_content = r#"echo "This is stdout"
echo "This is stderr" >&2
exit 0
"#;

    #[cfg(windows)]
    let script_content = r#"echo This is stdout
echo This is stderr 1>&2
exit /b 0
"#;

    create_fake_typst_in_temp(&temp_cache, version, script_content);

    let exec_options = ExecOptions {
        project_root: PathBuf::from("."),
        args: vec![],
        required_version: version.to_string(),
    };

    let exec_result = typstlab_typst::exec::exec_typst_with_override(
        exec_options,
        Some(temp_cache.path().to_path_buf()),
    )
    .unwrap();

    assert_eq!(exec_result.exit_code, 0);
    assert!(exec_result.stdout.contains("This is stdout"));
    assert!(exec_result.stderr.contains("This is stderr"));

    // TempDir automatically cleans up
}

/// Race condition regression tests
///
/// These tests verify that the ETXTBSY race condition is prevented by
/// the sync_all() + drop() fix applied to all test helpers.
#[cfg(test)]
mod race_condition_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;

    /// Test rapid creation and execution doesn't cause ETXTBSY
    ///
    /// This test creates and executes binaries in a tight loop to stress-test
    /// the file handle closing behavior. If sync_all() or drop() are missing,
    /// this test should fail with ETXTBSY on Linux/macOS.
    #[test]
    fn test_no_etxtbsy_race_condition() {
        let temp_cache = TempDir::new_in(std::env::current_dir().unwrap()).unwrap();
        let successes = Arc::new(Mutex::new(0));
        let errors = Arc::new(Mutex::new(Vec::new()));

        // Run 50 iterations to increase chance of catching race condition
        for i in 0..50 {
            let version = format!("0.{}.0", i);

            #[cfg(unix)]
            let script = "echo 'success'\nexit 0";
            #[cfg(windows)]
            let script = "echo success\r\nexit /b 0";

            // Create and immediately execute
            let _binary_path = create_fake_typst_in_temp(&temp_cache, &version, script);

            let exec_options = ExecOptions {
                project_root: PathBuf::from("."),
                args: vec!["--version".to_string()],
                required_version: version.clone(),
            };

            match typstlab_typst::exec::exec_typst_with_override(
                exec_options,
                Some(temp_cache.path().to_path_buf()),
            ) {
                Ok(result) => {
                    if result.exit_code == 0 {
                        let mut count = successes.lock().unwrap();
                        *count += 1;
                    } else {
                        let mut errs = errors.lock().unwrap();
                        errs.push(format!("Iteration {}: Exit code {}", i, result.exit_code));
                    }
                }
                Err(e) => {
                    let mut errs = errors.lock().unwrap();
                    errs.push(format!("Iteration {}: {}", i, e));
                }
            }
        }

        let success_count = *successes.lock().unwrap();
        let error_list = errors.lock().unwrap();

        // All iterations should succeed
        assert_eq!(
            success_count, 50,
            "Expected 50 successful executions, got {}. Errors: {:?}",
            success_count, *error_list
        );

        // TempDir automatically cleans up
    }

    /// Test parallel creation and execution (multi-threaded stress test)
    ///
    /// This test spawns multiple threads that simultaneously create and execute
    /// binaries. If file handle management is incorrect, this will likely fail
    /// with ETXTBSY or resource contention errors.
    #[test]
    fn test_no_race_condition_parallel() {
        let temp_cache = Arc::new(TempDir::new_in(std::env::current_dir().unwrap()).unwrap());
        let mut handles = vec![];

        // Spawn 4 threads that each create and execute 10 binaries
        for thread_id in 0..4 {
            let temp_cache_clone = Arc::clone(&temp_cache);

            let handle = thread::spawn(move || {
                let mut thread_errors = Vec::new();

                for i in 0..10 {
                    let version = format!("0.{}.{}", thread_id, i);

                    #[cfg(unix)]
                    let script = "echo 'success'\nexit 0";
                    #[cfg(windows)]
                    let script = "echo success\r\nexit /b 0";

                    let _binary_path =
                        create_fake_typst_in_temp(&temp_cache_clone, &version, script);

                    let exec_options = ExecOptions {
                        project_root: PathBuf::from("."),
                        args: vec!["--version".to_string()],
                        required_version: version.clone(),
                    };

                    if let Err(e) = typstlab_typst::exec::exec_typst_with_override(
                        exec_options,
                        Some(temp_cache_clone.path().to_path_buf()),
                    ) {
                        thread_errors.push(format!("Thread {} iteration {}: {}", thread_id, i, e));
                    }
                }

                thread_errors
            });

            handles.push(handle);
        }

        // Collect results
        let mut all_errors = Vec::new();
        for handle in handles {
            let thread_errors = handle.join().unwrap();
            all_errors.extend(thread_errors);
        }

        // No thread should have errors
        assert!(
            all_errors.is_empty(),
            "Parallel execution had errors: {:?}",
            all_errors
        );

        // TempDir automatically cleans up
    }

    /// Test immediate execution after creation (minimal delay)
    ///
    /// This test creates a binary and immediately executes it without any delay.
    /// This is the most likely scenario to trigger ETXTBSY if sync_all() or
    /// drop() are missing.
    #[test]
    fn test_immediate_execution_after_creation() {
        let temp_cache = TempDir::new_in(std::env::current_dir().unwrap()).unwrap();
        let version = "0.99.0";

        #[cfg(unix)]
        let script = "echo 'immediate'\nexit 0";
        #[cfg(windows)]
        let script = "echo immediate\r\nexit /b 0";

        // Create and immediately execute (no delay)
        let _binary_path = create_fake_typst_in_temp(&temp_cache, version, script);

        let exec_options = ExecOptions {
            project_root: PathBuf::from("."),
            args: vec!["--version".to_string()],
            required_version: version.to_string(),
        };

        let result = typstlab_typst::exec::exec_typst_with_override(
            exec_options,
            Some(temp_cache.path().to_path_buf()),
        );

        // Should succeed without ETXTBSY error
        assert!(
            result.is_ok(),
            "Immediate execution after creation failed: {:?}",
            result.err()
        );

        let exec_result = result.unwrap();
        assert_eq!(exec_result.exit_code, 0);

        // TempDir automatically cleans up
    }

    /// Test verify file is closed after persist (Unix-specific)
    ///
    /// This test uses lsof (if available) to verify the file descriptor is
    /// closed after persist(). This is a more direct test of the fix.
    #[cfg(unix)]
    #[test]
    fn test_file_handle_closed_after_persist() {
        use std::process::Command;

        let temp_cache = TempDir::new_in(std::env::current_dir().unwrap()).unwrap();
        let version = "0.98.0";
        let script = "echo 'test'\nexit 0";

        let binary_path = create_fake_typst_in_temp(&temp_cache, version, script);

        // Check if lsof is available
        let lsof_check = Command::new("which").arg("lsof").output();
        if lsof_check.is_err() || !lsof_check.unwrap().status.success() {
            // Skip test if lsof not available
            return;
        }

        // Check if binary file has any open file descriptors
        let lsof_output = Command::new("lsof")
            .arg(binary_path.to_str().unwrap())
            .output();

        match lsof_output {
            Ok(output) => {
                // If lsof succeeds but returns empty, file is not open (good)
                // If lsof succeeds with content, file has open handles (bad)
                let stdout = String::from_utf8_lossy(&output.stdout);

                if !stdout.is_empty() && stdout.contains(version) {
                    panic!(
                        "File handle not closed after persist! lsof output:\n{}",
                        stdout
                    );
                }
            }
            Err(_) => {
                // lsof failed (likely because file is not open) - this is good
                // Exit code 1 means "no files open" which is what we want
            }
        }

        // TempDir automatically cleans up
    }
}
