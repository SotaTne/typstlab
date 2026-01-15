//! Integration tests for `typstlab build` command

#![allow(deprecated)] // cargo_bin is deprecated but will be replaced in implementation phase

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use typstlab_testkit::{setup_test_typst, temp_dir_in_workspace, with_isolated_typst_env};

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
    fs::create_dir(root.join("dist")).unwrap();
    fs::create_dir(root.join(".typstlab")).unwrap();
}

/// Helper to create a test paper
fn create_test_paper(
    papers_dir: &std::path::Path,
    paper_id: &str,
    main_file: Option<&str>,
    root: Option<&str>,
) {
    let paper_dir = papers_dir.join(paper_id);
    fs::create_dir(&paper_dir).unwrap();

    let mut toml_content = format!(
        r#"
[paper]
id = "{}"
title = "Test Paper"
language = "en"
date = "2026-01-15"

[output]
name = "{}"
"#,
        paper_id, paper_id
    );

    // Add build section if custom main_file or root specified
    if main_file.is_some() || root.is_some() {
        toml_content.push_str("\n[build]\n");
        if let Some(file) = main_file {
            toml_content.push_str(&format!("main_file = \"{}\"\n", file));
        }
        if let Some(r) = root {
            toml_content.push_str(&format!("root = \"{}\"\n", r));
        }
    }

    fs::write(paper_dir.join("paper.toml"), toml_content).unwrap();
}

/// Helper to create main.typ file
fn create_main_file(paper_dir: &std::path::Path, path: &str, content: &str) {
    let full_path = paper_dir.join(path);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(full_path, content).unwrap();
}

#[test]
fn test_build_requires_project_root() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Don't create typstlab.toml - should fail

        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("build")
            .arg("--paper")
            .arg("paper1")
            .assert()
            .failure()
            .stderr(predicate::str::contains("project"));
    });
}

#[test]
fn test_build_requires_paper_id() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        // Should fail without --paper flag
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("build")
            .assert()
            .failure();
    });
}

#[test]
fn test_build_paper_not_found() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        // Build nonexistent paper
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("build")
            .arg("--paper")
            .arg("nonexistent")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not found").or(predicate::str::contains("nonexistent")),
            );
    });
}

#[test]
fn test_build_typst_not_resolved() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        let papers_dir = root.join("papers");
        create_test_paper(&papers_dir, "paper1", None, None);
        create_main_file(&papers_dir.join("paper1"), "main.typ", "= Test");

        // With environment isolation, typst is NOT available
        // Build should fail with "not found" or "not resolved" error
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("build")
            .arg("--paper")
            .arg("paper1")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not resolved").or(predicate::str::contains("not found")),
            );
    });
}

#[test]
fn test_build_main_file_not_found() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        let papers_dir = root.join("papers");
        create_test_paper(&papers_dir, "paper1", None, None);
        // Don't create main.typ

        // Install typst using setup_test_typst
        let typstlab_bin =
            std::path::PathBuf::from(Command::cargo_bin("typstlab").unwrap().get_program());
        let _typst_path = setup_test_typst(&typstlab_bin, root);

        // Build should fail because main.typ doesn't exist
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("build")
            .arg("--paper")
            .arg("paper1")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("main.typ")
                    .or(predicate::str::contains("not found"))
                    .or(predicate::str::contains("does not exist")),
            );
    });
}

#[test]
fn test_build_with_default_main_file() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        let papers_dir = root.join("papers");
        create_test_paper(&papers_dir, "paper1", None, None);
        create_main_file(&papers_dir.join("paper1"), "main.typ", "= Test Paper");

        // Install typst using setup_test_typst
        let typstlab_bin =
            std::path::PathBuf::from(Command::cargo_bin("typstlab").unwrap().get_program());
        let _typst_path = setup_test_typst(&typstlab_bin, root);

        // Build should succeed
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("build")
            .arg("--paper")
            .arg("paper1")
            .assert()
            .success();

        // Verify PDF created
        let pdf_path = root.join("dist/paper1/paper1.pdf");
        assert!(pdf_path.exists(), "PDF should be created at {:?}", pdf_path);
    });
}

#[test]
fn test_build_with_custom_main_file() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        let papers_dir = root.join("papers");
        create_test_paper(&papers_dir, "paper1", Some("custom.typ"), None);
        create_main_file(
            &papers_dir.join("paper1"),
            "custom.typ",
            "= Custom Main File",
        );

        // Install typst using setup_test_typst
        let typstlab_bin =
            std::path::PathBuf::from(Command::cargo_bin("typstlab").unwrap().get_program());
        let _typst_path = setup_test_typst(&typstlab_bin, root);

        // Build should succeed
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("build")
            .arg("--paper")
            .arg("paper1")
            .assert()
            .success();

        // Verify PDF created
        let pdf_path = root.join("dist/paper1/paper1.pdf");
        assert!(pdf_path.exists(), "PDF should be created at {:?}", pdf_path);
    });
}

#[test]
fn test_build_with_root_option() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        let papers_dir = root.join("papers");
        create_test_paper(&papers_dir, "paper1", Some("index.typ"), Some("src"));

        // Create src directory and index.typ
        let paper_dir = papers_dir.join("paper1");
        fs::create_dir(paper_dir.join("src")).unwrap();
        create_main_file(&paper_dir, "src/index.typ", "= Root Option Test");

        // Install typst using setup_test_typst
        let typstlab_bin =
            std::path::PathBuf::from(Command::cargo_bin("typstlab").unwrap().get_program());
        let _typst_path = setup_test_typst(&typstlab_bin, root);

        // Build should succeed
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("build")
            .arg("--paper")
            .arg("paper1")
            .assert()
            .success();

        // Verify PDF created
        let pdf_path = root.join("dist/paper1/paper1.pdf");
        assert!(pdf_path.exists(), "PDF should be created at {:?}", pdf_path);
    });
}

#[test]
fn test_build_root_dir_not_found() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        let papers_dir = root.join("papers");
        create_test_paper(
            &papers_dir,
            "paper1",
            Some("index.typ"),
            Some("nonexistent"),
        );
        // Don't create the "nonexistent" directory

        // Install typst using setup_test_typst
        let typstlab_bin =
            std::path::PathBuf::from(Command::cargo_bin("typstlab").unwrap().get_program());
        let _typst_path = setup_test_typst(&typstlab_bin, root);

        // Build should fail because root dir doesn't exist
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("build")
            .arg("--paper")
            .arg("paper1")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("nonexistent")
                    .or(predicate::str::contains("not found"))
                    .or(predicate::str::contains("does not exist")),
            );
    });
}

#[test]
fn test_build_with_full_flag() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        create_test_project(root, "0.12.0");

        let papers_dir = root.join("papers");
        create_test_paper(&papers_dir, "paper1", None, None);
        create_main_file(&papers_dir.join("paper1"), "main.typ", "= Full Flag Test");

        // Install typst using setup_test_typst
        let typstlab_bin =
            std::path::PathBuf::from(Command::cargo_bin("typstlab").unwrap().get_program());
        let _typst_path = setup_test_typst(&typstlab_bin, root);

        // Build with --full flag should succeed
        Command::cargo_bin("typstlab")
            .unwrap()
            .current_dir(root)
            .arg("build")
            .arg("--paper")
            .arg("paper1")
            .arg("--full")
            .assert()
            .success();

        // Verify PDF created
        let pdf_path = root.join("dist/paper1/paper1.pdf");
        assert!(pdf_path.exists(), "PDF should be created at {:?}", pdf_path);
    });
}
