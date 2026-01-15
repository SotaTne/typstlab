//! Integration tests for `typstlab new` command

#![allow(deprecated)]

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use typstlab_testkit::{temp_dir_in_workspace, with_isolated_typst_env};

#[test]
fn test_new_project_creates_structure() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("new")
            .arg("test-project")
            .assert()
            .success();

        let project_dir = root.join("test-project");

        // Verify directory structure
        assert!(project_dir.exists());
        assert!(project_dir.join("typstlab.toml").exists());
        assert!(project_dir.join(".gitignore").exists());
        assert!(project_dir.join("papers").is_dir());
        assert!(project_dir.join("layouts").is_dir());
        assert!(project_dir.join("refs").is_dir());
        assert!(project_dir.join("dist").is_dir());
        assert!(project_dir.join("rules").is_dir());
        assert!(project_dir.join(".typstlab").is_dir());
    });
}

#[test]
fn test_new_project_creates_valid_config() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("new")
            .arg("my-project")
            .assert()
            .success();

        let config_path = root.join("my-project/typstlab.toml");
        let config_content = fs::read_to_string(config_path).unwrap();

        // Verify config contains required sections
        assert!(config_content.contains("[project]"));
        assert!(config_content.contains("name = \"my-project\""));
        assert!(config_content.contains("[typst]"));
        assert!(config_content.contains("version ="));
    });
}

#[test]
fn test_new_project_creates_gitignore() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("new")
            .arg("test-project")
            .assert()
            .success();

        let gitignore_path = root.join("test-project/.gitignore");
        let gitignore_content = fs::read_to_string(gitignore_path).unwrap();

        // Verify common ignores
        assert!(gitignore_content.contains("dist/"));
        assert!(gitignore_content.contains("*.pdf"));
    });
}

#[test]
fn test_new_project_fails_if_exists() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Create project first time
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("new")
            .arg("test-project")
            .assert()
            .success();

        // Try to create again - should fail
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("new")
            .arg("test-project")
            .assert()
            .failure()
            .stderr(predicate::str::contains("exists").or(predicate::str::contains("already")));
    });
}

#[test]
fn test_new_paper_creates_structure() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Create project first
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

        let paper_dir = project_dir.join("papers/paper1");

        // Verify directory structure
        assert!(paper_dir.exists());
        assert!(paper_dir.join("paper.toml").exists());
        assert!(paper_dir.join("main.typ").exists());
        assert!(paper_dir.join("sections").is_dir());
        assert!(paper_dir.join("assets").is_dir());
        assert!(paper_dir.join("rules").is_dir());
        assert!(paper_dir.join("_generated").is_dir());
    });
}

#[test]
fn test_new_paper_creates_valid_config() {
    with_isolated_typst_env(None, |_cache| {
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
            .arg("my-paper")
            .assert()
            .success();

        let config_path = project_dir.join("papers/my-paper/paper.toml");
        let config_content = fs::read_to_string(config_path).unwrap();

        // Verify config contains required sections
        assert!(config_content.contains("[paper]"));
        assert!(config_content.contains("id = \"my-paper\""));
        assert!(config_content.contains("[output]"));
        assert!(config_content.contains("name ="));
    });
}

#[test]
fn test_new_paper_creates_main_typ() {
    with_isolated_typst_env(None, |_cache| {
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

        let main_typ_path = project_dir.join("papers/paper1/main.typ");
        let main_typ_content = fs::read_to_string(main_typ_path).unwrap();

        // Verify main.typ has basic structure
        assert!(main_typ_content.contains("#import"));
        assert!(!main_typ_content.is_empty());
    });
}

#[test]
fn test_new_paper_fails_if_not_in_project() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Don't create project - try to create paper directly
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("paper")
            .arg("new")
            .arg("paper1")
            .assert()
            .failure()
            .stderr(predicate::str::contains("project"));
    });
}

#[test]
fn test_new_paper_fails_if_exists() {
    with_isolated_typst_env(None, |_cache| {
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

        // Create paper first time
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(&project_dir)
            .arg("paper")
            .arg("new")
            .arg("paper1")
            .assert()
            .success();

        // Try to create again - should fail
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(&project_dir)
            .arg("paper")
            .arg("new")
            .arg("paper1")
            .assert()
            .failure()
            .stderr(predicate::str::contains("exists").or(predicate::str::contains("already")));
    });
}

#[test]
fn test_paper_list_empty_project() {
    with_isolated_typst_env(None, |_cache| {
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

        // List papers (should be empty)
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(&project_dir)
            .arg("paper")
            .arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("No papers found"));
    });
}

#[test]
fn test_paper_list_shows_papers() {
    with_isolated_typst_env(None, |_cache| {
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

        // Create two papers
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

        // List papers
        let output = Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(&project_dir)
            .arg("paper")
            .arg("list")
            .assert()
            .success();

        // Verify output contains both papers
        output
            .stdout(predicate::str::contains("paper1"))
            .stdout(predicate::str::contains("paper2"))
            .stdout(predicate::str::contains("Total: 2 paper(s)"));
    });
}

#[test]
fn test_paper_list_json_output() {
    with_isolated_typst_env(None, |_cache| {
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
            .arg("my-paper")
            .assert()
            .success();

        // List papers with JSON output
        let output = Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(&project_dir)
            .arg("paper")
            .arg("list")
            .arg("--json")
            .assert()
            .success();

        // Verify JSON structure
        output
            .stdout(predicate::str::contains("\"papers\""))
            .stdout(predicate::str::contains("\"count\""))
            .stdout(predicate::str::contains("\"id\": \"my-paper\""))
            .stdout(predicate::str::contains("\"title\""))
            .stdout(predicate::str::contains("\"language\""))
            .stdout(predicate::str::contains("\"date\""));
    });
}

#[test]
fn test_paper_list_fails_outside_project() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Don't create project - try to list papers directly
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("paper")
            .arg("list")
            .assert()
            .failure()
            .stderr(predicate::str::contains("project"));
    });
}

#[test]
fn test_new_project_rejects_path_traversal() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Try to create project with parent directory traversal
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("new")
            .arg("../../../etc/passwd")
            .assert()
            .failure()
            .stderr(predicate::str::contains("parent directory"));
    });
}

#[test]
fn test_new_project_rejects_absolute_path() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Try to create project with absolute path
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("new")
            .arg("/tmp/malicious")
            .assert()
            .failure()
            .stderr(predicate::str::contains("absolute path"));
    });
}

#[test]
fn test_new_project_rejects_empty_name() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Try to create project with empty name
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("new")
            .arg("")
            .assert()
            .failure()
            .stderr(predicate::str::contains("empty"));
    });
}

#[test]
fn test_new_paper_rejects_path_traversal() {
    with_isolated_typst_env(None, |_cache| {
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

        // Try to create paper with parent directory traversal
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(&project_dir)
            .arg("paper")
            .arg("new")
            .arg("../../../etc/shadow")
            .assert()
            .failure()
            .stderr(predicate::str::contains("parent directory"));
    });
}

#[test]
fn test_new_paper_rejects_absolute_path() {
    with_isolated_typst_env(None, |_cache| {
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

        // Try to create paper with absolute path
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(&project_dir)
            .arg("paper")
            .arg("new")
            .arg("/var/log/syslog")
            .assert()
            .failure()
            .stderr(predicate::str::contains("absolute path"));
    });
}
