//! End-to-end integration tests for typstlab-typst
//!
//! These tests verify the complete flow from binary resolution to command execution.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use typstlab_typst::{ExecOptions, ResolveOptions, ResolveResult};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Helper function to create a fake typst binary in a temp directory
fn create_fake_typst_in_temp(
    temp_dir: &TempDir,
    version: &str,
    script_content: &str
) -> PathBuf {
    let version_dir = temp_dir.path().join(version);
    fs::create_dir_all(&version_dir).unwrap();

    #[cfg(unix)]
    let binary_path = version_dir.join("typst");
    #[cfg(windows)]
    let binary_path = version_dir.join("typst.exe");

    #[cfg(unix)]
    {
        let script = format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  echo \"typst {}\"\n  exit 0\nfi\n{}",
            version, script_content
        );
        fs::write(&binary_path, script).unwrap();
        let mut perms = fs::metadata(&binary_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_path, perms).unwrap();
    }

    #[cfg(windows)]
    {
        let script = format!(
            "@echo off\nif \"%1\"==\"--version\" (\n  echo typst {}\n  exit /b 0\n)\n{}",
            version, script_content
        );
        fs::write(&binary_path, script).unwrap();
    }

    binary_path
}

/// Test complete flow: resolve managed binary -> execute command
#[test]
fn test_e2e_resolve_and_exec_managed() {
    let temp_cache = TempDir::new().unwrap();
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
        Some(temp_cache.path().to_path_buf())
    ).unwrap();

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
        Some(temp_cache.path().to_path_buf())
    ).unwrap();

    // Verify execution succeeded
    assert_eq!(exec_result.exit_code, 0);
    assert!(exec_result.stdout.contains("Hello from Typst"));
    assert!(exec_result.duration_ms > 0);

    // TempDir automatically cleans up
}

/// Test error case: binary not found
#[test]
fn test_e2e_binary_not_found() {
    let temp_cache = TempDir::new().unwrap();
    let version = "99.99.99"; // Non-existent version

    let exec_options = ExecOptions {
        project_root: PathBuf::from("."),
        args: vec!["compile".to_string(), "test.typ".to_string()],
        required_version: version.to_string(),
    };

    // Should fail because binary cannot be resolved
    let result = typstlab_typst::exec::exec_typst_with_override(
        exec_options,
        Some(temp_cache.path().to_path_buf())
    );
    assert!(result.is_err());

    // TempDir automatically cleans up
}

/// Test error case: binary execution fails
#[test]
fn test_e2e_execution_failure() {
    let temp_cache = TempDir::new().unwrap();
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
        Some(temp_cache.path().to_path_buf())
    ).unwrap();

    // Verify error was captured
    assert_eq!(exec_result.exit_code, 1);
    assert!(exec_result.stderr.contains("Error: File not found"));

    // TempDir automatically cleans up
}

/// Test force_refresh bypasses cache
#[test]
fn test_e2e_force_refresh() {
    let temp_cache = TempDir::new().unwrap();
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
        Some(temp_cache.path().to_path_buf())
    ).unwrap();
    assert!(matches!(result1, ResolveResult::Resolved(_)));

    // Second resolve with force_refresh should still work
    let resolve_options = ResolveOptions {
        required_version: version.to_string(),
        project_root: PathBuf::from("."),
        force_refresh: true,
    };

    let result2 = typstlab_typst::resolve::resolve_typst_with_override(
        resolve_options,
        Some(temp_cache.path().to_path_buf())
    ).unwrap();
    assert!(matches!(result2, ResolveResult::Resolved(_)));

    // TempDir automatically cleans up
}

/// Test execution with different exit codes
#[test]
fn test_e2e_various_exit_codes() {
    let temp_cache = TempDir::new().unwrap();
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
        Some(temp_cache.path().to_path_buf())
    ).unwrap();
    assert_eq!(exec_result.exit_code, 42);

    // TempDir automatically cleans up
}

/// Test stdout and stderr are properly captured
#[test]
fn test_e2e_output_capture() {
    let temp_cache = TempDir::new().unwrap();
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
        Some(temp_cache.path().to_path_buf())
    ).unwrap();

    assert_eq!(exec_result.exit_code, 0);
    assert!(exec_result.stdout.contains("This is stdout"));
    assert!(exec_result.stderr.contains("This is stderr"));

    // TempDir automatically cleans up
}
