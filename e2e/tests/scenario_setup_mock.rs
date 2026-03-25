use assert_cmd::assert::OutputAssertExt;
use std::fs;
use std::path::Path;
use typstlab_e2e_tests::{e2e_command, e2e_temp_dir};

/// Helper to create a mock typst binary in the managed cache
fn create_mock_typst(home: &Path, version: &str) {
    let cache_dir = home.join(".cache/typstlab");
    let version_dir = cache_dir.join(version);
    fs::create_dir_all(&version_dir).unwrap();

    #[cfg(unix)]
    let bin_path = version_dir.join("typst");
    #[cfg(windows)]
    let bin_path = version_dir.join("typst.exe");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let script = format!("#!/bin/sh\necho \"typst {}\"", version);
        fs::write(&bin_path, script).unwrap();
        let mut perms = fs::metadata(&bin_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin_path, perms).unwrap();
    }
    #[cfg(windows)]
    {
        fs::write(&bin_path, format!("@echo typst {}", version)).unwrap();
    }
}

#[test]
fn scenario_setup_with_mock_typst() {
    let temp = e2e_temp_dir();
    let root = temp.path();
    let project_dir = root.join("project-setup");
    fs::create_dir_all(&project_dir).unwrap();

    // 1. Init project
    e2e_command(&project_dir).arg("init").assert().success();

    // 2. Mock Typst 0.12.0 in the isolated home cache
    let isolated_home = root.join(".isolated-home");
    create_mock_typst(&isolated_home, "0.12.0");

    // 3. Run setup (should find the mock typst and create shim/state)
    e2e_command(&project_dir)
        .arg("setup")
        .arg("-v")
        .assert()
        .success();

    // 4. Verify results IN PROJECT ROOT
    assert!(project_dir.join("bin/typst").exists());
    assert!(project_dir.join(".typstlab/state.json").exists());

    let state_content = fs::read_to_string(project_dir.join(".typstlab/state.json")).unwrap();
    if !state_content.contains("managed") {
        panic!(
            "state.json does not contain 'managed'. Content:\n{}",
            state_content
        );
    }
    assert!(state_content.contains("0.12.0"));
}
