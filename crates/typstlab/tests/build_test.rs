//! Integration tests for `typstlab build` command

#![allow(deprecated)] // cargo_bin is deprecated but will be replaced in implementation phase

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use predicates::prelude::*;
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
}

#[test]
fn test_build_requires_paper_id() {
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
}

#[test]
fn test_build_paper_not_found() {
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
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("nonexistent")));
}

#[test]
fn test_build_typst_not_resolved() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    let papers_dir = root.join("papers");
    create_test_paper(&papers_dir, "paper1", None, None);
    create_main_file(&papers_dir.join("paper1"), "main.typ", "= Test");

    // Don't link typst - exec_typst will try to find system typst
    // If system typst is available, build will succeed
    // If not available, build will fail
    let result = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("build")
        .arg("--paper")
        .arg("paper1")
        .output()
        .unwrap();

    // Test passes if either:
    // 1. Build succeeds (system typst available)
    // 2. Build fails with "not resolved" error (no system typst)
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains("not resolved") || stderr.contains("not found"),
            "Expected 'not resolved' or 'not found' error, got: {}",
            stderr
        );
    }
    // If build succeeded, that's also OK (system typst was available)
}

#[test]
fn test_build_main_file_not_found() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    let papers_dir = root.join("papers");
    create_test_paper(&papers_dir, "paper1", None, None);
    // Don't create main.typ

    // Try to link first
    let link_result = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .output()
        .unwrap();

    // Skip test if link failed
    if !link_result.status.success() {
        eprintln!("Skipping test: system typst not available");
        return;
    }

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
}

#[test]
fn test_build_with_default_main_file() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    let papers_dir = root.join("papers");
    create_test_paper(&papers_dir, "paper1", None, None);
    create_main_file(&papers_dir.join("paper1"), "main.typ", "= Test Paper");

    // Try to link first
    let link_result = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .output()
        .unwrap();

    // Skip test if link failed
    if !link_result.status.success() {
        eprintln!("Skipping test: system typst not available");
        return;
    }

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
}

#[test]
fn test_build_with_custom_main_file() {
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

    // Try to link first
    let link_result = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .output()
        .unwrap();

    // Skip test if link failed
    if !link_result.status.success() {
        eprintln!("Skipping test: system typst not available");
        return;
    }

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
}

#[test]
fn test_build_with_root_option() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    let papers_dir = root.join("papers");
    create_test_paper(&papers_dir, "paper1", Some("index.typ"), Some("src"));

    // Create src directory and index.typ
    let paper_dir = papers_dir.join("paper1");
    fs::create_dir(paper_dir.join("src")).unwrap();
    create_main_file(&paper_dir, "src/index.typ", "= Root Option Test");

    // Try to link first
    let link_result = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .output()
        .unwrap();

    // Skip test if link failed
    if !link_result.status.success() {
        eprintln!("Skipping test: system typst not available");
        return;
    }

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
}

#[test]
fn test_build_root_dir_not_found() {
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

    // Try to link first
    let link_result = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .output()
        .unwrap();

    // Skip test if link failed
    if !link_result.status.success() {
        eprintln!("Skipping test: system typst not available");
        return;
    }

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
}

#[test]
fn test_build_with_full_flag() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    create_test_project(root, "0.12.0");

    let papers_dir = root.join("papers");
    create_test_paper(&papers_dir, "paper1", None, None);
    create_main_file(&papers_dir.join("paper1"), "main.typ", "= Full Flag Test");

    // Try to link first
    let link_result = Command::cargo_bin("typstlab")
        .unwrap()
        .current_dir(root)
        .arg("typst")
        .arg("link")
        .output()
        .unwrap();

    // Skip test if link failed
    if !link_result.status.success() {
        eprintln!("Skipping test: system typst not available");
        return;
    }

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
}
