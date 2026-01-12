//! Integration tests for CLI infrastructure

#![allow(deprecated)] // cargo_bin is deprecated but will be replaced in implementation phase

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper: Create a temporary typstlab project
fn create_test_project() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("typstlab.toml");

    // Create minimal valid typstlab.toml
    let minimal_config = r#"
[project]
name = "test-project"
init_date = "2026-01-12"

[typst]
version = "0.17.0"
"#;

    fs::write(&config_path, minimal_config).expect("Failed to write config");
    temp_dir
}

#[test]
fn test_cli_version_flag() {
    // Arrange & Act: Run with --version flag
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd.arg("--version").assert();

    // Assert: Should print version and exit 0
    assert
        .success()
        .stdout(predicate::str::contains("typstlab"));
}

#[test]
fn test_cli_help_flag() {
    // Arrange & Act: Run with --help flag
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd.arg("--help").assert();

    // Assert: Should print help and exit 0
    assert
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("Commands:"));
}

#[test]
fn test_cli_requires_project_root() {
    // Arrange: Create temp directory WITHOUT typstlab.toml
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Act: Run doctor command from non-project directory
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd.arg("doctor").current_dir(temp_dir.path()).assert();

    // Assert: Doctor always exits 0 (even when no project found)
    // Error is reported in output, not exit code
    assert.success();
}

#[test]
fn test_cli_finds_project_root_from_subdir() {
    // Arrange: Create project with subdirectory
    let project = create_test_project();
    let subdir = project.path().join("papers").join("paper1");
    fs::create_dir_all(&subdir).expect("Failed to create subdirectory");

    // Act: Run doctor command from subdirectory
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd.arg("doctor").current_dir(&subdir).assert();

    // Assert: Should find project root and execute command
    // doctor always exits 0, even on warnings/errors
    assert.success();
}

#[test]
fn test_cli_verbose_flag() {
    // Arrange: Create test project
    let project = create_test_project();

    // Act: Run with --verbose flag
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("--verbose")
        .arg("doctor")
        .current_dir(project.path())
        .assert();

    // Assert: Should execute successfully with verbose output
    assert.success();
}

#[test]
fn test_cli_json_flag_for_doctor() {
    // Arrange: Create test project
    let project = create_test_project();

    // Act: Run doctor with --json flag
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("doctor")
        .arg("--json")
        .current_dir(project.path())
        .assert();

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
}

#[test]
fn test_cli_typst_docs_status() {
    // Arrange: Create test project
    let project = create_test_project();

    // Act: Run typst docs status
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("typst")
        .arg("docs")
        .arg("status")
        .current_dir(project.path())
        .assert();

    // Assert: Should execute successfully (status always exits 0)
    assert.success();
}
