use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Helper to get the typstlab binary path
pub fn cargo_bin_path() -> PathBuf {
    if let Ok(exe) = std::env::var("CARGO_BIN_EXE_typstlab") {
        return PathBuf::from(exe);
    }

    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();

    let mut debug_bin = workspace_root.join("target/debug/typstlab");
    if cfg!(windows) {
        debug_bin.set_extension("exe");
    }

    if debug_bin.exists() {
        return debug_bin;
    }

    panic!("Could not find typstlab binary at {}", debug_bin.display());
}

/// Helper to create a temp directory in tests/tmp/ at workspace root
pub fn e2e_temp_dir() -> TempDir {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let tmp_root = workspace_root.join("e2e/tmp");
    if !tmp_root.exists() {
        fs::create_dir_all(&tmp_root).unwrap();
    }

    tempfile::Builder::new()
        .prefix("typstlab-scenario-")
        .tempdir_in(tmp_root)
        .unwrap()
}

/// Helper to create a Command with isolated environment variables
pub fn e2e_command(current_dir: &Path) -> Command {
    let bin_path = cargo_bin_path();
    // Use a home directory that is NOT inside current_dir to avoid interference with project files,
    // but keep it within the temp root.
    let isolated_home = current_dir.parent().unwrap().join(".isolated-home");
    fs::create_dir_all(&isolated_home).unwrap();

    let mut cmd = Command::new(bin_path);
    cmd.current_dir(current_dir)
        .env("HOME", &isolated_home)
        .env("USERPROFILE", &isolated_home)
        .env("TYPSTLAB_CACHE_DIR", isolated_home.join(".cache/typstlab"));

    cmd
}
