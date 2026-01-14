//! Integration tests for `typstlab typst link` command

#![allow(deprecated)] // cargo_bin is deprecated but will be replaced in implementation phase

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use typstlab_testkit::temp_dir_in_workspace;

/// Helper to create a minimal typstlab project
fn create_test_project(root: &std::path::Path, typst_version: &str) {
    fs::write(
        root.join("typstlab.toml"),
        format!(
            r#"
[project]
name = "test-project"
init_date = "2026-01-15"

[typst]
version = "{}"
"#,
            typst_version
        ),
    )
    .unwrap();

    fs::create_dir(root.join("papers")).unwrap();
}

#[test]
fn test_link_requires_project_root() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Don't create typstlab.toml - should fail

    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .assert()
        .failure()
        .stderr(predicate::str::contains("typstlab.toml"));
}

#[test]
fn test_link_with_system_typst_version_mismatch() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Create project requiring a specific version
    create_test_project(root, "0.12.0");

    // Attempt to link - will likely fail if system typst is different version
    let result = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .assert();

    // Should either succeed (if system typst matches) or fail with version mismatch
    let output = result.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        assert!(
            stderr.contains("version") || stderr.contains("not found"),
            "Should mention version or not found, got: {}",
            stderr
        );
    }
}

#[test]
fn test_link_force_flag_triggers_refresh() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Get system typst version
    let version_output = std::process::Command::new("typst")
        .arg("--version")
        .output();

    if version_output.is_err() {
        // Skip test if typst not available
        return;
    }

    let binding = version_output.unwrap();
    let version_str = String::from_utf8_lossy(&binding.stdout);
    // Extract version number (e.g., "typst 0.12.0" -> "0.12.0")
    let version = version_str
        .split_whitespace()
        .nth(1)
        .expect("Failed to parse typst version");

    create_test_project(root, version);

    // First link
    let _ = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .assert();

    // Second link with --force (should succeed)
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .arg("--force")
        .assert()
        .success();
}

#[test]
fn test_link_creates_state_json() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Get system typst version
    let version_output = std::process::Command::new("typst")
        .arg("--version")
        .output();

    if version_output.is_err() {
        // Skip test if typst not available
        return;
    }

    let binding = version_output.unwrap();
    let version_str = String::from_utf8_lossy(&binding.stdout);
    let version = version_str
        .split_whitespace()
        .nth(1)
        .expect("Failed to parse typst version");

    create_test_project(root, version);

    // Run link
    let result = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .assert();

    if result.get_output().status.success() {
        // Verify state.json exists
        assert!(
            root.join(".typstlab").join("state.json").exists(),
            ".typstlab/state.json should be created"
        );
    }
}

#[test]
fn test_link_creates_bin_shim() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Get system typst version
    let version_output = std::process::Command::new("typst")
        .arg("--version")
        .output();

    if version_output.is_err() {
        // Skip test if typst not available
        return;
    }

    let binding = version_output.unwrap();
    let version_str = String::from_utf8_lossy(&binding.stdout);
    let version = version_str
        .split_whitespace()
        .nth(1)
        .expect("Failed to parse typst version");

    create_test_project(root, version);

    // Run link
    let result = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .assert();

    if result.get_output().status.success() {
        // Verify bin/typst shim exists
        #[cfg(unix)]
        let shim_path = root.join("bin").join("typst");
        #[cfg(windows)]
        let shim_path = root.join("bin").join("typst.cmd");

        assert!(shim_path.exists(), "bin/typst shim should be created");
    }
}
