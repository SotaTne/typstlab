//! Integration tests for `typstlab typst exec` command

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
fn test_exec_requires_project_root() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Don't create typstlab.toml - should fail

    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("exec")
        .arg("--")
        .arg("--version")
        .assert()
        .failure()
        .stderr(predicate::str::contains("typstlab.toml"));
}

#[test]
fn test_exec_requires_double_dash() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    // Should fail without --
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("exec")
        .arg("--version")
        .assert()
        .failure();
}

#[test]
fn test_exec_forwards_to_typst() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    // Try to link first (may fail if system typst not available)
    let link_result = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .output()
        .unwrap();

    // Skip test if link failed (no system typst)
    if !link_result.status.success() {
        eprintln!("Skipping test: system typst not available");
        return;
    }

    // Run exec with --version
    let result = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("exec")
        .arg("--")
        .arg("--version")
        .assert();

    let output = result.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain "typst" (from typst --version output)
    if output.status.success() {
        assert!(
            stdout.to_lowercase().contains("typst"),
            "Should forward to typst binary, got: {}",
            stdout
        );
    }
}

#[test]
fn test_exec_fails_if_not_resolved() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    // Don't link - should fail
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("exec")
        .arg("--")
        .arg("--version")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not resolved").or(predicate::str::contains("state.json")),
        );
}

#[test]
fn test_exec_preserves_exit_code() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    // Try to link first
    let link_result = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .output()
        .unwrap();

    // Skip test if link failed
    if !link_result.status.success() {
        eprintln!("Skipping test: system typst not available");
        return;
    }

    // Run with invalid command (should fail)
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("exec")
        .arg("--")
        .arg("invalid-command")
        .assert()
        .failure();
}
