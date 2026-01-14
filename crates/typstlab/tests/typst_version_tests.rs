//! Integration tests for `typstlab typst version` command

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
fn test_version_requires_project_root() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Don't create typstlab.toml - should fail

    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("version")
        .assert()
        .failure()
        .stderr(predicate::str::contains("typstlab.toml"));
}

#[test]
fn test_version_shows_required_version() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    // Should show required version even if not resolved
    let result = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("version")
        .assert();

    let output = result.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain "required: 0.12.0"
    assert!(
        stdout.contains("0.12.0") || stdout.contains("required"),
        "Should show required version, got: {}",
        stdout
    );
}

#[test]
fn test_version_json_output() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    let result = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("version")
        .arg("--json")
        .assert();

    let output = result.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should be valid JSON
    if output.status.success() {
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!(
                "Should output valid JSON, got error: {}, output: {}",
                e, stdout
            )
        });

        // Should have required_version field
        assert!(
            json.get("required_version").is_some(),
            "JSON should have required_version field, got: {}",
            json
        );
    }
}

#[test]
fn test_version_with_resolved_typst() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    // Try to link first (may fail if system typst not available)
    let _ = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .output();

    // Run version command
    let result = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("version")
        .assert();

    let output = result.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain required version
    assert!(
        stdout.contains("0.12.0") || stdout.contains("required"),
        "Should show version info, got: {}",
        stdout
    );
}
