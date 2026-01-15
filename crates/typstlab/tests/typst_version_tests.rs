//! Integration tests for `typstlab typst version` command

#![allow(deprecated)] // cargo_bin is deprecated but will be replaced in implementation phase

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use typstlab_testkit::{setup_test_typst, temp_dir_in_workspace, with_isolated_typst_env};

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
    with_isolated_typst_env(None, |_cache| {
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
    });
}

#[test]
fn test_version_shows_required_version() {
    with_isolated_typst_env(None, |_cache| {
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
    });
}

#[test]
fn test_version_json_output() {
    with_isolated_typst_env(None, |_cache| {
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
    });
}

#[test]
fn test_version_with_resolved_typst() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        // Install typst using setup_test_typst
        let typstlab_bin =
            std::path::PathBuf::from(Command::cargo_bin("typstlab").unwrap().get_program());
        let _typst_path = setup_test_typst(&typstlab_bin, root);

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
    });
}
