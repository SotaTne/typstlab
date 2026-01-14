//! Integration tests for `typstlab status` command

#![allow(deprecated)]

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use typstlab_testkit::temp_dir_in_workspace;

#[test]
fn test_status_pass_all_checks() {
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

    // Create paper with valid structure
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("paper")
        .arg("new")
        .arg("paper1")
        .assert()
        .success();

    // Run status command
    let output = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("status")
        .assert()
        .success(); // Exit code 0 always

    // Verify output contains checks
    output
        .stdout(predicate::str::contains("environment check"))
        .stdout(predicate::str::contains("typst check"))
        .stdout(predicate::str::contains("build check"))
        .stdout(predicate::str::contains("refs check"));
}

#[test]
fn test_status_with_paper_filter() {
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

    // Run status with paper filter
    let output = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("status")
        .arg("--paper")
        .arg("paper1")
        .assert()
        .success();

    // Verify output mentions paper1
    output.stdout(predicate::str::contains("paper1"));
}

#[test]
fn test_status_json_output() {
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

    // Run status with --json flag
    let output = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("status")
        .arg("--json")
        .assert()
        .success();

    // Verify JSON structure
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(json["schema_version"], "1.0");
    assert!(json["project"].is_object());
    assert!(json["timestamp"].is_string());
    assert!(json["overall_status"].is_string());
    assert!(json["checks"].is_array());
    assert!(json["actions"].is_array());
}

#[test]
fn test_status_human_output() {
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

    // Run status (default human output)
    let output = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("status")
        .assert()
        .success();

    // Verify human-readable format with status icons
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    // Should contain check or x marks (✓ or ✗)
    assert!(stdout.contains("check") || stdout.contains("Check"));

    // Should not be JSON
    assert!(!stdout.contains("\"schema_version\""));
}

#[test]
fn test_status_exit_code_always_zero() {
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

    // Create paper1
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("paper")
        .arg("new")
        .arg("paper1")
        .assert()
        .success();

    // Delete main.typ to cause build check error
    let paper_dir = project_dir.join("papers").join("paper1");
    fs::remove_file(paper_dir.join("main.typ")).unwrap();

    // Run status - should exit 0 even with errors
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(&project_dir)
        .arg("status")
        .assert()
        .success(); // Exit code 0 (per DESIGN.md 5.1 Exit Code Policy)
}

#[test]
fn test_status_fails_outside_project() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Don't create project - try to run status directly
    Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("status")
        .assert()
        .failure() // Should fail because not in project
        .stderr(predicate::str::contains("project"));
}
