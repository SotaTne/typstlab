//! Test utilities for typstlab
//!
//! This crate provides shared testing utilities used across the typstlab workspace.

use std::path::PathBuf;

// Module declarations
mod env;
mod fixtures;
mod mock;
mod temp;

// Re-export public API from submodules
pub use env::{ENV_LOCK, with_isolated_typst_env};
pub use fixtures::{setup_test_typst, setup_typst_from_fixtures};
pub use mock::{SHARED_MOCK_SERVER, get_shared_mock_server, init_shared_mock_github_url};
pub use temp::{temp_dir_in_workspace, try_temp_dir_in_workspace};

/// Get the path to a compiled example binary
///
/// This helper locates example binaries compiled by cargo test.
/// Example binaries are in the `target/debug/examples/` directory.
///
/// # Arguments
///
/// * `name` - Name of the example binary (without .exe extension)
///
/// # Returns
///
/// PathBuf to the compiled example binary
///
/// # Panics
///
/// Panics if unable to determine the current executable path
///
/// # Examples
///
/// ```no_run
/// use typstlab_testkit::example_bin;
/// use std::process::Command;
///
/// // Example test function (not executed in doctest)
/// fn test_with_example() {
///     let status = Command::new(example_bin("counter_child"))
///         .arg("counter.txt")
///         .arg("10")
///         .status()
///         .unwrap();
///     assert!(status.success());
/// }
/// ```
pub fn example_bin(name: &str) -> PathBuf {
    let mut path = std::env::current_exe().expect("Failed to get current executable path");

    // Navigate from target/debug/deps/test_binary to target/debug/examples/
    path.pop(); // Remove test binary name
    path.pop(); // Remove "deps"
    path.push("examples");
    path.push(name);

    // Add .exe extension on Windows
    if cfg!(target_os = "windows") {
        path.set_extension("exe");
    }

    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_bin_returns_correct_path() {
        let path = example_bin("test_example");

        // Verify path contains "examples" directory
        assert!(
            path.to_string_lossy().contains("examples"),
            "Path should contain 'examples' directory"
        );

        // Verify path ends with the binary name
        let file_name = path.file_name().unwrap().to_string_lossy();
        assert!(
            file_name.starts_with("test_example"),
            "File name should start with 'test_example'"
        );

        // Verify .exe extension on Windows
        #[cfg(target_os = "windows")]
        assert!(
            file_name.ends_with(".exe"),
            "File should have .exe extension on Windows"
        );

        // Verify no .exe extension on Unix
        #[cfg(not(target_os = "windows"))]
        assert!(
            !file_name.ends_with(".exe"),
            "File should not have .exe extension on Unix"
        );
    }
}
