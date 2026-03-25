use assert_cmd::assert::OutputAssertExt;
use std::fs;
use typstlab_e2e_tests::{e2e_command, e2e_temp_dir};

#[test]
fn scenario_init_existing_directory() {
    let temp = e2e_temp_dir();
    let root = temp.path();

    // Create an existing directory
    let target = root.join("existing-dir");
    fs::create_dir_all(&target).unwrap();

    // Run init command with isolated home
    e2e_command(&target)
        .arg("init")
        .assert()
        .success();

    assert!(target.join("typstlab.toml").exists());
}
