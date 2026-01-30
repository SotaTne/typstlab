use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo_bin;
use predicates::prelude::*;
use std::process::Command;
use typstlab_testkit::{temp_dir_in_workspace, with_isolated_typst_env};

#[test]
fn test_gen_paper_in_project() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        // Create project
        Command::new(cargo_bin!("typstlab"))
            .current_dir(root)
            .arg("new")
            .arg("myproject")
            .assert()
            .success();

        let project_dir = root.join("myproject");

        // Run gen paper. Note: current dir must be the project dir.
        Command::new(cargo_bin!("typstlab"))
            .current_dir(&project_dir)
            .arg("gen")
            .arg("paper")
            .arg("mypaper")
            .assert()
            .success();

        // Verify files
        let paper_dir = project_dir.join("papers").join("mypaper");
        assert!(paper_dir.exists());
        assert!(paper_dir.join("paper.toml").exists());
        assert!(paper_dir.join("main.typ").exists());

        // Test with --title
        Command::new(cargo_bin!("typstlab"))
            .current_dir(&project_dir)
            .arg("gen")
            .arg("paper")
            .arg("titled-paper")
            .arg("--title")
            .arg("My Custom Title")
            .assert()
            .success();

        let titled_toml = project_dir.join("papers/titled-paper/paper.toml");
        let toml_content = std::fs::read_to_string(titled_toml).unwrap();
        assert!(toml_content.contains("title = \"My Custom Title\""));
    });
}

#[test]
fn test_gen_layout_in_project() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        Command::new(cargo_bin!("typstlab"))
            .current_dir(root)
            .arg("new")
            .arg("layout-project")
            .assert()
            .success();

        let project_dir = root.join("layout-project");

        // Run gen layout
        Command::new(cargo_bin!("typstlab"))
            .current_dir(&project_dir)
            .arg("gen")
            .arg("layout")
            .arg("custom-layout")
            .assert()
            .success();

        // Verify files
        let layout_dir = project_dir.join("layouts").join("custom-layout");
        assert!(layout_dir.exists());
        assert!(layout_dir.join("meta.tmp.typ").exists());
        assert!(layout_dir.join("header.typ").exists());
    });
}

#[test]
fn test_gen_lib_stub() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        Command::new(cargo_bin!("typstlab"))
            .current_dir(root)
            .arg("new")
            .arg("lib-project")
            .assert()
            .success();

        let project_dir = root.join("lib-project");

        // Run gen lib (stub)
        Command::new(cargo_bin!("typstlab"))
            .current_dir(&project_dir)
            .arg("gen")
            .arg("lib")
            .arg("mylib")
            .assert()
            .success()
            .stdout(predicate::str::contains("not yet implemented"))
            .stdout(predicate::str::contains("v0.2"));
    });
}

#[test]
fn test_paper_list_admin_command() {
    with_isolated_typst_env(None, |_cache| {
        let temp = temp_dir_in_workspace();
        let root = temp.path();

        Command::new(cargo_bin!("typstlab"))
            .current_dir(root)
            .arg("new")
            .arg("list-project")
            .arg("--paper")
            .arg("p1")
            .assert()
            .success();

        let project_dir = root.join("list-project");

        // Run paper list
        Command::new(cargo_bin!("typstlab"))
            .current_dir(&project_dir)
            .arg("paper")
            .arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("p1"))
            .stdout(predicate::str::contains("Papers in project"));
    });
}
