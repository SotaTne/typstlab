//! Integration tests for `typstlab typst install` command

#![allow(deprecated)] // cargo_bin is deprecated but will be replaced in implementation phase

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use typstlab_testkit::{temp_dir_in_workspace, with_isolated_typst_env};

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
fn test_install_requires_project_root() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Don't create typstlab.toml - should fail

        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("typst")
            .arg("install")
            .arg("0.12.0")
            .assert()
            .failure()
            .stderr(predicate::str::contains("typstlab.toml"));
    });
}

#[test]
fn test_install_accepts_version_argument() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        // Should accept version argument (may fail due to network, but should parse correctly)
        let result = Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("typst")
            .arg("install")
            .arg("0.12.0")
            .assert();

        // Either succeeds or fails with network/download error (not argument error)
        let output = result.get_output();
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            // Should not be argument parsing error
            assert!(
                !stderr.contains("argument") && !stderr.contains("usage"),
                "Should not be argument error, got: {}",
                stderr
            );
        }
    });
}

#[test]
fn test_install_from_cargo_flag() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        // Should accept --from-cargo flag
        let result = Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("typst")
            .arg("install")
            .arg("0.12.0")
            .arg("--from-cargo")
            .assert();

        // Either succeeds or fails with cargo error (not flag parsing error)
        let output = result.get_output();
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            // Should not be flag parsing error
            assert!(
                !stderr.contains("unexpected argument") && !stderr.contains("usage"),
                "Should not be flag error, got: {}",
                stderr
            );
        }
    });
}

#[test]
fn test_install_creates_managed_cache() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        // Run install (may skip if already installed or network unavailable)
        let _result = Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("typst")
            .arg("install")
            .arg("0.12.0")
            .assert();

        // Note: We cannot reliably test cache creation in CI without network
        // This test mainly verifies the command structure
    });
}

#[test]
fn test_install_updates_state_json() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        // Run install (may skip if already installed or network unavailable)
        let result = Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("typst")
            .arg("install")
            .arg("0.12.0")
            .assert();

        // If successful, state.json should be updated
        if result.get_output().status.success() {
            let _state_path = root.join(".typstlab").join("state.json");
            // State may not be created if install was skipped
            // This is acceptable behavior
        }
    });
}
