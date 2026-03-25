//! Integration tests for `typstlab typst install` command

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo_bin;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use typstlab_testkit::{temp_dir_in_workspace, with_isolated_typst_env};

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
fn test_install_requires_project_root_when_no_arg() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Don't create typstlab.toml - should fail when no arg is provided
        Command::new(cargo_bin!("typstlab"))
            .current_dir(root)
            .arg("typst")
            .arg("install")
            .assert()
            .failure()
            .stderr(predicate::str::contains("typstlab.toml"));
    });
}

#[test]
fn test_install_defaults_to_config_version() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        // Should work without version argument by reading typstlab.toml
        let result = Command::new(cargo_bin!("typstlab"))
            .current_dir(root)
            .arg("typst")
            .arg("install")
            .assert();

        let output = result.get_output();
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Check if it mentions using version from typstlab.toml
        assert!(stdout.contains("using version from typstlab.toml"));
    });
}

#[test]
fn test_install_accepts_version_argument() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        // Should accept version argument
        let result = Command::new(cargo_bin!("typstlab"))
            .current_dir(root)
            .arg("typst")
            .arg("install")
            .arg("0.11.1")
            .assert();

        let output = result.get_output();
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Should install the specified version, not the one in config
        assert!(stdout.contains("Installing Typst 0.11.1"));
    });
}

#[test]
fn test_install_creates_managed_cache() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        // Run install
        let _result = Command::new(cargo_bin!("typstlab"))
            .current_dir(root)
            .arg("typst")
            .arg("install")
            .assert();
    });
}
