use assert_cmd::assert::OutputAssertExt;
use predicates::prelude::*;
use typstlab_e2e_tests::{e2e_command, e2e_temp_dir};

#[test]
fn scenario_new_project_creation() {
    let temp = e2e_temp_dir();
    let root = temp.path();

    // Run new project command with isolated home
    e2e_command(root)
        .arg("new")
        .arg("project1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created project 'project1'"));

    let project_dir = root.join("project1");
    
    // Verify project files
    assert!(project_dir.join("typstlab.toml").exists());
    assert!(project_dir.join("templates/default/main.tmp.typ").exists());
    assert!(project_dir.join("templates/default/template.typ").exists());
}
