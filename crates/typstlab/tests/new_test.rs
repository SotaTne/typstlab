//! Integration tests for `typstlab new` and `init` commands

#![allow(deprecated)]

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use typstlab_testkit::{temp_dir_in_workspace, with_isolated_typst_env};

#[test]
fn test_new_project_defaults_to_empty() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("new")
            .arg("empty-project")
            .assert()
            .success();

        let project_dir = root.join("empty-project");

        // Verify directory structure
        assert!(project_dir.exists());
        assert!(project_dir.join("typstlab.toml").exists());
        assert!(project_dir.join(".gitignore").exists());

        // Should have papers directory but it should be empty
        assert!(project_dir.join("papers").is_dir());
        assert!(
            fs::read_dir(project_dir.join("papers"))
                .unwrap()
                .next()
                .is_none()
        );
    });
}

#[test]
fn test_new_project_with_paper_flag() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("new")
            .arg("paper-project")
            .arg("--paper")
            .arg("report")
            .assert()
            .success();

        let project_dir = root.join("paper-project");
        assert!(project_dir.exists());

        // Verify paper was created
        assert!(project_dir.join("papers/report/paper.toml").exists());
        assert!(project_dir.join("papers/report/main.typ").exists());
    });
}

#[test]
fn test_init_project_in_current_dir() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();
        let project_dir = root.join("init-project");
        fs::create_dir(&project_dir).unwrap();

        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(&project_dir)
            .arg("init")
            .assert()
            .success();

        // Verify initialization
        assert!(project_dir.join("typstlab.toml").exists());
        assert!(project_dir.join(".gitignore").exists());
        assert!(project_dir.join("papers").is_dir());
    });
}

#[test]
fn test_init_project_with_paper_flag() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();
        let project_dir = root.join("init-paper-project");
        fs::create_dir(&project_dir).unwrap();

        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(&project_dir)
            .arg("init")
            .arg("--paper")
            .arg("thesis")
            .assert()
            .success();

        // Verify paper was created
        assert!(project_dir.join("papers/thesis/paper.toml").exists());
    });
}

#[test]
fn test_new_project_fails_if_exists() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("new")
            .arg("existing-project")
            .assert()
            .success();

        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("new")
            .arg("existing-project")
            .assert()
            .failure()
            .stderr(predicate::str::contains("exists"));
    });
}

#[test]
fn test_init_fails_if_already_initialized() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();
        let project_dir = root.join("double-init");
        fs::create_dir(&project_dir).unwrap();

        // First init
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(&project_dir)
            .arg("init")
            .assert()
            .success();

        // Second init - should fail
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(&project_dir)
            .arg("init")
            .assert()
            .failure()
            .stderr(predicate::str::contains("already"));
    });
}
