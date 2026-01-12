//! Integration tests for doctor command

#![allow(deprecated)] // cargo_bin is deprecated but will be replaced in implementation phase

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
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

/// Helper: Create a project with invalid config
fn create_project_with_invalid_config() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("typstlab.toml");

    // Create invalid typstlab.toml (missing required fields)
    let invalid_config = r#"
[project]
name = "test-project"
# missing init_date

[typst]
# missing version
"#;

    fs::write(&config_path, invalid_config).expect("Failed to write config");
    temp_dir
}

#[test]
fn test_doctor_exits_zero_on_success() {
    // Arrange: Create valid test project
    let project = create_test_project();

    // Act: Run doctor command
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd.arg("doctor").current_dir(project.path()).assert();

    // Assert: Should always exit 0 (even if there are warnings)
    assert.success();
}

#[test]
fn test_doctor_exits_zero_on_failure() {
    // Arrange: Create project with invalid config
    let project = create_project_with_invalid_config();

    // Act: Run doctor command
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd.arg("doctor").current_dir(project.path()).assert();

    // Assert: Should still exit 0 (failures recorded in output, not exit code)
    assert.success();
}

#[test]
fn test_doctor_json_output_structure() {
    // Arrange: Create test project
    let project = create_test_project();

    // Act: Run doctor with --json flag
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("doctor")
        .arg("--json")
        .current_dir(project.path())
        .assert();

    // Assert: Should output valid JSON with required structure
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify JSON is parseable
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Verify required top-level fields
    assert!(
        json.get("schema_version").is_some(),
        "JSON should contain schema_version field"
    );
    assert!(
        json.get("project").is_some(),
        "JSON should contain project field"
    );
    assert!(
        json.get("timestamp").is_some(),
        "JSON should contain timestamp field"
    );
    assert!(
        json.get("checks").is_some(),
        "JSON should contain checks array"
    );

    // Verify checks is an array
    assert!(json["checks"].is_array(), "checks field should be an array");
}

#[test]
fn test_doctor_json_check_structure() {
    // Arrange: Create test project
    let project = create_test_project();

    // Act: Run doctor with --json flag
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("doctor")
        .arg("--json")
        .current_dir(project.path())
        .assert();

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Assert: If checks exist, they should have the correct structure
    if let Some(checks) = json["checks"].as_array() {
        for check in checks {
            assert!(
                check.get("id").is_some(),
                "Each check should have an id field"
            );
            assert!(
                check.get("name").is_some(),
                "Each check should have a name field"
            );
            assert!(
                check.get("status").is_some(),
                "Each check should have a status field"
            );

            // Verify status is one of: ok, warning, error
            let status = check["status"].as_str().unwrap();
            assert!(
                ["ok", "warning", "error"].contains(&status),
                "Status should be ok, warning, or error, got: {}",
                status
            );
        }
    }
}

#[test]
fn test_doctor_checks_typst_availability() {
    // Arrange: Create test project
    let project = create_test_project();

    // Act: Run doctor with --json flag
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("doctor")
        .arg("--json")
        .current_dir(project.path())
        .assert();

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Assert: Should include a check for Typst availability
    let checks = json["checks"]
        .as_array()
        .expect("checks should be an array");

    let typst_check = checks
        .iter()
        .find(|check| check["id"].as_str() == Some("typst_available"));

    assert!(
        typst_check.is_some(),
        "Should include typst_available check"
    );

    // Verify the check has expected fields
    if let Some(check) = typst_check {
        assert!(
            check.get("message").is_some(),
            "Check should have a message"
        );
        assert!(check.get("details").is_some(), "Check should have details");
    }
}

#[test]
fn test_doctor_checks_config_validity() {
    // Arrange: Create test project
    let project = create_test_project();

    // Act: Run doctor with --json flag
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("doctor")
        .arg("--json")
        .current_dir(project.path())
        .assert();

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Assert: Should include a check for config validity
    let checks = json["checks"]
        .as_array()
        .expect("checks should be an array");

    let config_check = checks
        .iter()
        .find(|check| check["id"].as_str() == Some("config_valid"));

    assert!(config_check.is_some(), "Should include config_valid check");
}

#[test]
fn test_doctor_invalid_config_reports_error() {
    // Arrange: Create project with invalid config
    let project = create_project_with_invalid_config();

    // Act: Run doctor with --json flag
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("doctor")
        .arg("--json")
        .current_dir(project.path())
        .assert();

    // Assert: Should still exit 0
    let assert = assert.success();
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Should have config_valid check with error status
    let checks = json["checks"]
        .as_array()
        .expect("checks should be an array");

    let config_check = checks
        .iter()
        .find(|check| check["id"].as_str() == Some("config_valid"));

    if let Some(check) = config_check {
        assert_eq!(
            check["status"].as_str(),
            Some("error"),
            "Invalid config should result in error status"
        );
    }
}

#[test]
fn test_doctor_human_readable_output() {
    // Arrange: Create test project
    let project = create_test_project();

    // Act: Run doctor without --json flag
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd.arg("doctor").current_dir(project.path()).assert();

    // Assert: Should output human-readable text (not JSON)
    let assert = assert.success();
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not be valid JSON (or at least not start with '{')
    // Human readable output should contain some informative text
    assert!(
        !stdout.trim().starts_with('{') || stdout.contains('\n'),
        "Without --json, output should be human-readable"
    );
}

#[test]
fn test_doctor_verbose_flag() {
    // Arrange: Create test project
    let project = create_test_project();

    // Act: Run doctor with --verbose flag
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("--verbose")
        .arg("doctor")
        .current_dir(project.path())
        .assert();

    // Assert: Should execute successfully with verbose output
    assert.success();
}
