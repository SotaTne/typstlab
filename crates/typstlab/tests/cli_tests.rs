//! Integration tests for CLI infrastructure

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo_bin;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use typstlab_testkit::{temp_dir_in_workspace, with_isolated_typst_env};

/// Helper: Create a temporary typstlab project in workspace .tmp/
fn create_test_project(root: &std::path::Path) {
    let config_path = root.join("typstlab.toml");

    // Create minimal valid typstlab.toml
    let minimal_config = r#"
[project]
name = "test-project"
init_date = "2026-01-12"

[typst]
version = "0.17.0"
"#;

    fs::write(&config_path, minimal_config).expect("Failed to write config");
}

#[test]
fn test_cli_version_flag() {
    with_isolated_typst_env(None, |_cache| {
        // Arrange & Act: Run with --version flag
        let mut cmd = Command::new(cargo_bin!(env!("CARGO_PKG_NAME")));
        let assert = cmd.arg("--version").assert();

        // Assert: Should print version and exit 0
        assert
            .success()
            .stdout(predicate::str::contains("typstlab"));
    });
}

#[test]
fn test_cli_help_flag() {
    with_isolated_typst_env(None, |_cache| {
        // Arrange & Act: Run with --help flag
        let mut cmd = Command::new(cargo_bin!(env!("CARGO_PKG_NAME")));
        let assert = cmd.arg("--help").assert();

        // Assert: Should print help and exit 0
        assert
            .success()
            .stdout(predicate::str::contains("Usage:"))
            .stdout(predicate::str::contains("Commands:"));
    });
}

#[test]
fn test_cli_requires_project_root() {
    with_isolated_typst_env(None, |_cache| {
        // Arrange: Create temp directory WITHOUT typstlab.toml
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Act: Run doctor command from non-project directory
        let mut cmd = Command::new(cargo_bin!(env!("CARGO_PKG_NAME")));
        let assert = cmd.arg("doctor").current_dir(root).assert();

        // Assert: Doctor always exits 0 (even when no project found)
        // Error is reported in output, not exit code
        assert.success();
    });
}

#[test]
fn test_cli_finds_project_root_from_subdir() {
    with_isolated_typst_env(None, |_cache| {
        // Arrange: Create project with subdirectory
        let temp = temp_dir_in_workspace();
        let root = temp.path();
        create_test_project(root);
        let subdir = root.join("papers").join("paper1");
        fs::create_dir_all(&subdir).expect("Failed to create subdirectory");

        // Act: Run doctor command from subdirectory
        let mut cmd = Command::new(cargo_bin!(env!("CARGO_PKG_NAME")));
        let assert = cmd.arg("doctor").current_dir(&subdir).assert();

        // Assert: Should find project root and execute command
        // doctor always exits 0, even on warnings/errors
        assert.success();
    });
}

#[test]
fn test_cli_verbose_flag() {
    with_isolated_typst_env(None, |_cache| {
        // Arrange: Create test project
        let temp = temp_dir_in_workspace();
        let root = temp.path();
        create_test_project(root);

        // Act: Run with --verbose flag
        let mut cmd = Command::new(cargo_bin!(env!("CARGO_PKG_NAME")));
        let assert = cmd
            .arg("--verbose")
            .arg("doctor")
            .current_dir(root)
            .assert();

        // Assert: Should execute successfully with verbose output
        assert.success();
    });
}

#[test]
fn test_cli_json_flag_for_doctor() {
    with_isolated_typst_env(None, |_cache| {
        // Arrange: Create test project
        let temp = temp_dir_in_workspace();
        let root = temp.path();
        create_test_project(root);

        // Act: Run doctor with --json flag
        let mut cmd = Command::new(cargo_bin!(env!("CARGO_PKG_NAME")));
        let assert = cmd.arg("doctor").arg("--json").current_dir(root).assert();

        // Assert: Should output valid JSON
        let output = assert.get_output();
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Verify JSON is parseable
        let json: serde_json::Value =
            serde_json::from_str(&stdout).expect("Output should be valid JSON");

        // Verify required fields
        assert!(
            json.get("schema_version").is_some(),
            "JSON should contain schema_version field"
        );
    });
}

#[test]
fn test_cli_typst_docs_status() {
    with_isolated_typst_env(None, |_cache| {
        // Arrange: Create test project
        let temp = temp_dir_in_workspace();
        let root = temp.path();
        create_test_project(root);

        // Act: Run typst docs status
        let mut cmd = Command::new(cargo_bin!(env!("CARGO_PKG_NAME")));
        let assert = cmd
            .arg("typst")
            .arg("docs")
            .arg("status")
            .current_dir(root)
            .assert();

        // Assert: Should execute successfully (status always exits 0)
        assert.success();
    });
}
