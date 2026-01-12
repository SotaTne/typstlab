//! Integration tests for typst docs commands

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

    // Create minimal valid typstlab.toml with network policy "auto"
    let minimal_config = r#"
[project]
name = "test-project"
init_date = "2026-01-12"

[typst]
version = "0.12.0"

[network]
policy = "auto"
"#;

    fs::write(&config_path, minimal_config).expect("Failed to write config");
    temp_dir
}

/// Helper: Create a project with network policy "never"
fn create_project_with_network_never() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("typstlab.toml");

    let config_with_never = r#"
[project]
name = "test-project"
init_date = "2026-01-12"

[typst]
version = "0.12.0"

[network]
policy = "never"
"#;

    fs::write(&config_path, config_with_never).expect("Failed to write config");
    temp_dir
}

#[test]
fn test_docs_status_before_sync() {
    // Arrange: Create test project
    let project = create_test_project();

    // Act: Run docs status before any sync
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("typst")
        .arg("docs")
        .arg("status")
        .current_dir(project.path())
        .assert();

    // Assert: Should exit 0 (status always succeeds)
    let assert = assert.success();
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Before sync, docs should not be present
    assert!(
        stdout.contains("not present") || stdout.contains("false"),
        "Status should indicate docs not present before sync"
    );
}

#[test]
fn test_docs_status_json_structure() {
    // Arrange: Create test project
    let project = create_test_project();

    // Act: Run docs status with --json flag
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("typst")
        .arg("docs")
        .arg("status")
        .arg("--json")
        .current_dir(project.path())
        .assert();

    // Assert: Should output valid JSON
    let assert = assert.success();
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify JSON is parseable
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Verify required fields
    assert!(
        json.get("present").is_some(),
        "JSON should contain 'present' field"
    );
    assert!(
        json.get("version").is_some(),
        "JSON should contain 'version' field"
    );
}

#[test]
fn test_docs_sync_downloads_docs() {
    // Arrange: Create test project
    let project = create_test_project();
    let docs_dir = project.path().join(".typstlab/kb/typst/docs");

    // Verify docs directory doesn't exist initially
    assert!(
        !docs_dir.exists(),
        "Docs directory should not exist before sync"
    );

    // Act: Run docs sync
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("typst")
        .arg("docs")
        .arg("sync")
        .current_dir(project.path())
        .assert();

    // Assert: Should succeed
    assert.success();

    // Verify docs directory was created
    assert!(
        docs_dir.exists(),
        "Docs directory should exist after successful sync"
    );

    // Verify at least some files were extracted
    let entries: Vec<_> = fs::read_dir(&docs_dir)
        .expect("Should be able to read docs directory")
        .collect();

    assert!(
        !entries.is_empty(),
        "Docs directory should contain files after sync"
    );
}

#[test]
fn test_docs_status_after_sync() {
    // Arrange: Create test project and sync docs
    let project = create_test_project();

    let mut sync_cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    sync_cmd
        .arg("typst")
        .arg("docs")
        .arg("sync")
        .current_dir(project.path())
        .assert()
        .success();

    // Act: Check status after sync
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("typst")
        .arg("docs")
        .arg("status")
        .current_dir(project.path())
        .assert();

    // Assert: Should show docs as present
    let assert = assert.success();
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("present") || stdout.contains("true"),
        "Status should indicate docs are present after sync"
    );
}

#[test]
fn test_docs_clear_removes_docs() {
    // Arrange: Create test project, sync docs, verify they exist
    let project = create_test_project();
    let docs_dir = project.path().join(".typstlab/kb/typst/docs");

    let mut sync_cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    sync_cmd
        .arg("typst")
        .arg("docs")
        .arg("sync")
        .current_dir(project.path())
        .assert()
        .success();

    assert!(docs_dir.exists(), "Docs directory should exist after sync");

    // Act: Run docs clear
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("typst")
        .arg("docs")
        .arg("clear")
        .current_dir(project.path())
        .assert();

    // Assert: Should succeed
    assert.success();

    // Verify docs directory was removed
    assert!(
        !docs_dir.exists(),
        "Docs directory should not exist after clear"
    );
}

#[test]
fn test_docs_status_after_clear() {
    // Arrange: Sync then clear docs
    let project = create_test_project();

    // Sync first
    let mut sync_cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    sync_cmd
        .arg("typst")
        .arg("docs")
        .arg("sync")
        .current_dir(project.path())
        .assert()
        .success();

    // Clear docs
    let mut clear_cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    clear_cmd
        .arg("typst")
        .arg("docs")
        .arg("clear")
        .current_dir(project.path())
        .assert()
        .success();

    // Act: Check status after clear
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("typst")
        .arg("docs")
        .arg("status")
        .current_dir(project.path())
        .assert();

    // Assert: Should show docs as not present
    let assert = assert.success();
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("not present") || stdout.contains("false"),
        "Status should indicate docs not present after clear"
    );
}

#[test]
fn test_docs_sync_respects_network_policy_never() {
    // Arrange: Create project with network policy "never"
    let project = create_project_with_network_never();
    let docs_dir = project.path().join(".typstlab/kb/typst/docs");

    // Act: Attempt to sync with network policy "never"
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("typst")
        .arg("docs")
        .arg("sync")
        .current_dir(project.path())
        .assert();

    // Assert: Should fail (exit non-zero)
    let assert = assert.failure();
    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should mention network policy in error message
    assert!(
        stderr.contains("network") || stderr.contains("policy"),
        "Error should mention network policy restriction"
    );

    // Docs should not be downloaded
    assert!(
        !docs_dir.exists(),
        "Docs should not be downloaded when network policy is 'never'"
    );
}

#[test]
fn test_docs_sync_updates_state_json() {
    // Arrange: Create test project
    let project = create_test_project();
    let state_path = project.path().join(".typstlab/state.json");

    // Act: Run docs sync
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.arg("typst")
        .arg("docs")
        .arg("sync")
        .current_dir(project.path())
        .assert()
        .success();

    // Assert: state.json should be created and contain docs info
    assert!(state_path.exists(), "state.json should exist after sync");

    let state_content = fs::read_to_string(&state_path).expect("Should read state.json");
    let state: serde_json::Value =
        serde_json::from_str(&state_content).expect("state.json should be valid JSON");

    // Verify docs.typst field exists and has required properties
    let docs_typst = state
        .get("docs")
        .and_then(|d| d.get("typst"))
        .expect("state.json should contain docs.typst");

    assert!(
        docs_typst.get("present").is_some(),
        "docs.typst should have 'present' field"
    );
    assert!(
        docs_typst.get("version").is_some(),
        "docs.typst should have 'version' field"
    );
    assert!(
        docs_typst.get("synced_at").is_some(),
        "docs.typst should have 'synced_at' field"
    );
    assert!(
        docs_typst.get("source").is_some(),
        "docs.typst should have 'source' field"
    );
}

#[test]
fn test_docs_clear_updates_state_json() {
    // Arrange: Sync docs first
    let project = create_test_project();
    let state_path = project.path().join(".typstlab/state.json");

    let mut sync_cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    sync_cmd
        .arg("typst")
        .arg("docs")
        .arg("sync")
        .current_dir(project.path())
        .assert()
        .success();

    // Act: Clear docs
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.arg("typst")
        .arg("docs")
        .arg("clear")
        .current_dir(project.path())
        .assert()
        .success();

    // Assert: state.json should reflect docs as not present
    let state_content = fs::read_to_string(&state_path).expect("Should read state.json");
    let state: serde_json::Value =
        serde_json::from_str(&state_content).expect("state.json should be valid JSON");

    let docs_typst = state
        .get("docs")
        .and_then(|d| d.get("typst"))
        .expect("state.json should contain docs.typst");

    let present = docs_typst
        .get("present")
        .and_then(|p| p.as_bool())
        .expect("present should be a boolean");

    assert!(
        !present,
        "docs.typst.present should be false after clear"
    );
}

#[test]
fn test_docs_verbose_flag() {
    // Arrange: Create test project
    let project = create_test_project();

    // Act: Run docs sync with --verbose flag
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("--verbose")
        .arg("typst")
        .arg("docs")
        .arg("sync")
        .current_dir(project.path())
        .assert();

    // Assert: Should execute successfully with verbose output
    assert.success();
}
