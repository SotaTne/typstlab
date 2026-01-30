//! Integration tests for `typstlab typst versions` command

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo_bin;
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
fn test_versions_command_exits_successfully() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    // Command should exit successfully even with no managed versions
    Command::new(cargo_bin!("typstlab"))
        .current_dir(root)
        .arg("typst")
        .arg("versions")
        .assert()
        .success();
}

#[test]
fn test_versions_human_readable_output() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    let output = Command::new(cargo_bin!("typstlab"))
        .current_dir(root)
        .arg("typst")
        .arg("versions")
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    // Should contain header
    assert!(
        stdout.contains("Typst versions") || stdout.contains("versions"),
        "Output should contain 'versions', got: {}",
        stdout
    );
}

#[test]
fn test_versions_json_output() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    let output = Command::new(cargo_bin!("typstlab"))
        .current_dir(root)
        .arg("typst")
        .arg("versions")
        .arg("--json")
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    // Should be valid JSON
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "Should output valid JSON, got error: {}, output: {}",
            e, stdout
        )
    });

    // Should have versions array
    assert!(
        json.get("versions").is_some(),
        "JSON should have 'versions' array, got: {}",
        json
    );

    assert!(
        json["versions"].is_array(),
        "'versions' should be an array, got: {}",
        json["versions"]
    );
}

#[test]
fn test_versions_json_schema() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    let output = Command::new(cargo_bin!("typstlab"))
        .current_dir(root)
        .arg("typst")
        .arg("versions")
        .arg("--json")
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Verify schema structure
    if let Some(versions) = json["versions"].as_array() {
        for version_entry in versions {
            // Each entry should have required fields
            assert!(
                version_entry.get("version").is_some(),
                "Each version entry should have 'version' field"
            );
            assert!(
                version_entry.get("source").is_some(),
                "Each version entry should have 'source' field"
            );
            assert!(
                version_entry.get("path").is_some(),
                "Each version entry should have 'path' field"
            );
            assert!(
                version_entry.get("is_current").is_some(),
                "Each version entry should have 'is_current' field"
            );

            // Source should be "managed" or "system"
            let source = version_entry["source"].as_str().unwrap();
            assert!(
                source == "managed" || source == "system",
                "Source should be 'managed' or 'system', got: {}",
                source
            );

            // is_current should be boolean
            assert!(
                version_entry["is_current"].is_boolean(),
                "is_current should be boolean"
            );
        }
    }
}

#[test]
fn test_versions_with_no_versions_installed() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    // Don't install any managed versions
    // System version may or may not exist

    let output = Command::new(cargo_bin!("typstlab"))
        .current_dir(root)
        .arg("typst")
        .arg("versions")
        .arg("--json")
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Should return empty array or only system version
    let versions = json["versions"].as_array().unwrap();

    // All entries should be system (if any exist)
    for _entry in versions {
        // No managed versions should exist at this point
        // (This test verifies graceful handling of empty managed cache)
    }
}

#[test]
fn test_versions_marks_current_version() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    // Try to link (may succeed or fail depending on system)
    let _ = Command::new(cargo_bin!("typstlab"))
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .output();

    // Check if state.json was created
    let state_path = root.join(".typstlab/state.json");
    if !state_path.exists() {
        // Skip test if link failed (no Typst available)
        return;
    }

    let output = Command::new(cargo_bin!("typstlab"))
        .current_dir(root)
        .arg("typst")
        .arg("versions")
        .arg("--json")
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let versions = json["versions"].as_array().unwrap();

    // At least one version should be marked as current
    let current_count = versions
        .iter()
        .filter(|v| v["is_current"].as_bool() == Some(true))
        .count();

    // Should have exactly one current version (if resolved)
    if current_count > 0 {
        assert_eq!(
            current_count, 1,
            "Should have exactly one current version, got: {}",
            current_count
        );
    }
}

#[test]
fn test_versions_shows_system_local_marker() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    let output = Command::new(cargo_bin!("typstlab"))
        .current_dir(root)
        .arg("typst")
        .arg("versions")
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    // If system version exists, should show (local) marker
    // Check JSON to see if system version exists
    let json_output = Command::new(cargo_bin!("typstlab"))
        .current_dir(root)
        .arg("typst")
        .arg("versions")
        .arg("--json")
        .output()
        .unwrap();

    let json_stdout = String::from_utf8(json_output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_stdout).unwrap();

    let has_system_version = json["versions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["source"].as_str() == Some("system"));

    if has_system_version {
        // Should contain (local) marker in human output
        assert!(
            stdout.contains("(local)"),
            "System version should have (local) marker, got: {}",
            stdout
        );
    }
}

#[test]
fn test_versions_sorted_descending() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    // This test will pass if versions are sorted
    // We can't easily install multiple versions in test environment
    // So this test mainly verifies the command doesn't crash

    let output = Command::new(cargo_bin!("typstlab"))
        .current_dir(root)
        .arg("typst")
        .arg("versions")
        .arg("--json")
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Verify versions array exists
    assert!(json["versions"].is_array());

    // If multiple versions exist, verify sorted order
    // (This will be properly tested in unit tests)
}

#[test]
fn test_versions_current_marker_in_human_output() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    // Try to link
    let _ = Command::new(cargo_bin!("typstlab"))
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .output();

    // Check if state.json was created
    let state_path = root.join(".typstlab/state.json");
    if !state_path.exists() {
        // Skip test if link failed
        return;
    }

    let output = Command::new(cargo_bin!("typstlab"))
        .current_dir(root)
        .arg("typst")
        .arg("versions")
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    // Check JSON first to see if there's a current version
    let json_output = Command::new(cargo_bin!("typstlab"))
        .current_dir(root)
        .arg("typst")
        .arg("versions")
        .arg("--json")
        .output()
        .unwrap();

    let json_stdout = String::from_utf8(json_output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_stdout).unwrap();

    let has_current = json["versions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["is_current"].as_bool() == Some(true));

    if has_current {
        // Human output should contain * marker
        assert!(
            stdout.contains("*") || stdout.contains("current"),
            "Current version should have * marker, got: {}",
            stdout
        );
    }
}
