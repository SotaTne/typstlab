//! Integration tests for `typstlab sync` command

#![allow(deprecated)]

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use typstlab_testkit::temp_dir_in_workspace;

#[test]
fn test_sync_default_mode() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Create project
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("new")
        .arg("test-project")
        .assert()
        .success();

    let project_dir = root.join("test-project");

    // Create paper
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("paper")
        .arg("new")
        .arg("paper1")
        .assert()
        .success();

    // Run sync (default mode)
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("sync")
        .assert()
        .success();

    // Verify state.json updated with last_sync
    let state_path = project_dir.join(".typstlab/state.json");
    assert!(state_path.exists(), "state.json should exist");

    let state_content = fs::read_to_string(&state_path).unwrap();
    let state: serde_json::Value = serde_json::from_str(&state_content).unwrap();

    assert!(
        state["sync"]["last_sync"].is_string(),
        "sync.last_sync should be a timestamp string"
    );

    // Verify _generated/ exists
    let generated_dir = project_dir.join("papers/paper1/_generated");
    assert!(
        generated_dir.exists(),
        "_generated/ should be created by sync"
    );
}

#[test]
fn test_sync_with_multiple_papers() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Create project with two papers
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("new")
        .arg("test-project")
        .assert()
        .success();

    let project_dir = root.join("test-project");

    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("paper")
        .arg("new")
        .arg("paper1")
        .assert()
        .success();

    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("paper")
        .arg("new")
        .arg("paper2")
        .assert()
        .success();

    // Run sync
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("sync")
        .assert()
        .success();

    // Verify both papers' _generated/ exist
    assert!(
        project_dir.join("papers/paper1/_generated").exists(),
        "paper1 _generated/ should exist"
    );
    assert!(
        project_dir.join("papers/paper2/_generated").exists(),
        "paper2 _generated/ should exist"
    );
}

#[test]
fn test_sync_idempotency() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Create project
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("new")
        .arg("test-project")
        .assert()
        .success();

    let project_dir = root.join("test-project");

    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("paper")
        .arg("new")
        .arg("paper1")
        .assert()
        .success();

    // Run sync first time
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("sync")
        .assert()
        .success();

    let state_path = project_dir.join(".typstlab/state.json");
    let first_state = fs::read_to_string(&state_path).unwrap();
    let first_json: serde_json::Value = serde_json::from_str(&first_state).unwrap();
    let first_sync = first_json["sync"]["last_sync"].as_str().unwrap();

    // Run sync second time
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("sync")
        .assert()
        .success();

    // Verify state.json updated (timestamp changed)
    let second_state = fs::read_to_string(&state_path).unwrap();
    let second_json: serde_json::Value = serde_json::from_str(&second_state).unwrap();
    let second_sync = second_json["sync"]["last_sync"].as_str().unwrap();

    // Timestamps should be different (second run updated)
    assert_ne!(
        first_sync, second_sync,
        "Idempotent operations should update timestamp"
    );

    // But structure should remain valid
    assert!(project_dir.join("papers/paper1/_generated").exists());
}

#[test]
fn test_sync_exit_code() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Create valid project
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("new")
        .arg("test-project")
        .assert()
        .success();

    let project_dir = root.join("test-project");

    // Sync should succeed (exit 0)
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("sync")
        .assert()
        .success();
}

#[test]
fn test_sync_fails_outside_project() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Don't create project - try to run sync directly
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("sync")
        .assert()
        .failure() // Should fail because not in project
        .stderr(predicate::str::contains("project"));
}

#[test]
fn test_sync_state_json_updated() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Create project
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("new")
        .arg("test-project")
        .assert()
        .success();

    let project_dir = root.join("test-project");

    // Run sync
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("sync")
        .assert()
        .success();

    // Verify state.json structure
    let state_path = project_dir.join(".typstlab/state.json");
    let state_content = fs::read_to_string(&state_path).unwrap();
    let state: serde_json::Value = serde_json::from_str(&state_content).unwrap();

    // Check sync field exists and has correct structure
    assert!(state["sync"].is_object(), "sync should be an object");
    assert!(
        state["sync"]["last_sync"].is_string(),
        "last_sync should be ISO 8601 timestamp"
    );

    // Parse timestamp to verify it's valid ISO 8601
    let timestamp_str = state["sync"]["last_sync"].as_str().unwrap();
    let _parsed = chrono::DateTime::parse_from_rfc3339(timestamp_str)
        .expect("last_sync should be valid ISO 8601");
}

#[test]
fn test_sync_generates_layouts() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Create project
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("new")
        .arg("test-project")
        .assert()
        .success();

    let project_dir = root.join("test-project");

    // Create paper
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("paper")
        .arg("new")
        .arg("paper1")
        .assert()
        .success();

    // Run sync
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("sync")
        .assert()
        .success();

    // Verify generated files exist
    let generated_dir = project_dir.join("papers/paper1/_generated");
    assert!(generated_dir.join("meta.typ").exists());
    assert!(generated_dir.join("header.typ").exists());
    assert!(generated_dir.join("refs.typ").exists());
}

#[test]
fn test_sync_output_format() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Create project
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("new")
        .arg("test-project")
        .assert()
        .success();

    let project_dir = root.join("test-project");

    // Run sync and check output
    let output = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("sync")
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    // Verify human-readable output contains expected messages
    assert!(
        stdout.contains("Resolving") || stdout.contains("Generating") || stdout.contains("Sync"),
        "Output should contain progress messages"
    );
}
