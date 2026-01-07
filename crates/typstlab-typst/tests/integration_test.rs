//! End-to-end integration tests for typstlab-typst
//!
//! These tests verify the complete flow from binary resolution to command execution.

use std::fs;
use std::path::PathBuf;
use typstlab_typst::{exec_typst, resolve_typst, ExecOptions, ResolveOptions, ResolveResult};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Helper function to create a fake typst binary for testing
#[cfg(unix)]
fn create_fake_typst_binary(path: &std::path::Path, version: &str, script_content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();

    let script = format!(
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "typst {}"
  exit 0
fi
{}
"#,
        version, script_content
    );

    fs::write(path, script).unwrap();
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

#[cfg(windows)]
fn create_fake_typst_binary(path: &std::path::Path, version: &str, script_content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();

    let script = format!(
        r#"@echo off
if "%1"=="--version" (
  echo typst {}
  exit /b 0
)
{}
"#,
        version, script_content
    );

    fs::write(path, script).unwrap();
}

/// Test complete flow: resolve managed binary -> execute command
#[test]
fn test_e2e_resolve_and_exec_managed() {
    let version = "0.18.0";
    let cache_dir = typstlab_typst::resolve::managed_cache_dir().unwrap();
    let version_dir = cache_dir.join(version);

    #[cfg(unix)]
    let binary_path = version_dir.join("typst");
    #[cfg(windows)]
    let binary_path = version_dir.join("typst.exe");

    // Create fake binary that outputs "Hello from Typst"
    let script_content = r#"echo "Hello from Typst"
exit 0
"#;
    create_fake_typst_binary(&binary_path, version, script_content);

    // Step 1: Resolve the binary
    let resolve_options = ResolveOptions {
        required_version: version.to_string(),
        project_root: PathBuf::from("."),
        force_refresh: false,
    };

    let resolve_result = resolve_typst(resolve_options).unwrap();

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

    let exec_result = exec_typst(exec_options).unwrap();

    // Verify execution succeeded
    assert_eq!(exec_result.exit_code, 0);
    assert!(exec_result.stdout.contains("Hello from Typst"));
    assert!(exec_result.duration_ms > 0);

    // Cleanup
    let _ = fs::remove_dir_all(&version_dir);
}

/// Test error case: binary not found
#[test]
fn test_e2e_binary_not_found() {
    let version = "99.99.99"; // Non-existent version

    let exec_options = ExecOptions {
        project_root: PathBuf::from("."),
        args: vec!["compile".to_string(), "test.typ".to_string()],
        required_version: version.to_string(),
    };

    // Should fail because binary cannot be resolved
    let result = exec_typst(exec_options);
    assert!(result.is_err());
}

/// Test error case: binary execution fails
#[test]
fn test_e2e_execution_failure() {
    let version = "0.19.0";
    let cache_dir = typstlab_typst::resolve::managed_cache_dir().unwrap();
    let version_dir = cache_dir.join(version);

    #[cfg(unix)]
    let binary_path = version_dir.join("typst");
    #[cfg(windows)]
    let binary_path = version_dir.join("typst.exe");

    // Create fake binary that exits with error code
    let script_content = r#"echo "Error: File not found" >&2
exit 1
"#;
    create_fake_typst_binary(&binary_path, version, script_content);

    let exec_options = ExecOptions {
        project_root: PathBuf::from("."),
        args: vec!["compile".to_string(), "nonexistent.typ".to_string()],
        required_version: version.to_string(),
    };

    let exec_result = exec_typst(exec_options).unwrap();

    // Verify error was captured
    assert_eq!(exec_result.exit_code, 1);
    assert!(exec_result.stderr.contains("Error: File not found"));

    // Cleanup
    let _ = fs::remove_dir_all(&version_dir);
}

/// Test force_refresh bypasses cache
#[test]
fn test_e2e_force_refresh() {
    let version = "0.20.0";
    let cache_dir = typstlab_typst::resolve::managed_cache_dir().unwrap();
    let version_dir = cache_dir.join(version);

    #[cfg(unix)]
    let binary_path = version_dir.join("typst");
    #[cfg(windows)]
    let binary_path = version_dir.join("typst.exe");

    create_fake_typst_binary(&binary_path, version, "exit 0");

    // First resolve without force_refresh
    let resolve_options = ResolveOptions {
        required_version: version.to_string(),
        project_root: PathBuf::from("."),
        force_refresh: false,
    };

    let result1 = resolve_typst(resolve_options).unwrap();
    assert!(matches!(result1, ResolveResult::Resolved(_)));

    // Second resolve with force_refresh should still work
    let resolve_options = ResolveOptions {
        required_version: version.to_string(),
        project_root: PathBuf::from("."),
        force_refresh: true,
    };

    let result2 = resolve_typst(resolve_options).unwrap();
    assert!(matches!(result2, ResolveResult::Resolved(_)));

    // Cleanup
    let _ = fs::remove_dir_all(&version_dir);
}

/// Test execution with different exit codes
#[test]
fn test_e2e_various_exit_codes() {
    let version = "0.21.0";
    let cache_dir = typstlab_typst::resolve::managed_cache_dir().unwrap();
    let version_dir = cache_dir.join(version);

    #[cfg(unix)]
    let binary_path = version_dir.join("typst");
    #[cfg(windows)]
    let binary_path = version_dir.join("typst.exe");

    // Test exit code 42
    let script_content = "exit 42";
    create_fake_typst_binary(&binary_path, version, script_content);

    let exec_options = ExecOptions {
        project_root: PathBuf::from("."),
        args: vec!["test".to_string()],
        required_version: version.to_string(),
    };

    let exec_result = exec_typst(exec_options).unwrap();
    assert_eq!(exec_result.exit_code, 42);

    // Cleanup
    let _ = fs::remove_dir_all(&version_dir);
}

/// Test stdout and stderr are properly captured
#[test]
fn test_e2e_output_capture() {
    let version = "0.22.0";
    let cache_dir = typstlab_typst::resolve::managed_cache_dir().unwrap();
    let version_dir = cache_dir.join(version);

    #[cfg(unix)]
    let binary_path = version_dir.join("typst");
    #[cfg(windows)]
    let binary_path = version_dir.join("typst.exe");

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

    create_fake_typst_binary(&binary_path, version, script_content);

    let exec_options = ExecOptions {
        project_root: PathBuf::from("."),
        args: vec![],
        required_version: version.to_string(),
    };

    let exec_result = exec_typst(exec_options).unwrap();

    assert_eq!(exec_result.exit_code, 0);
    assert!(exec_result.stdout.contains("This is stdout"));
    assert!(exec_result.stderr.contains("This is stderr"));

    // Cleanup
    let _ = fs::remove_dir_all(&version_dir);
}
