//! Test utilities for typstlab
//!
//! This crate provides shared testing utilities used across the typstlab workspace.

use tempfile::TempDir;

/// Creates a temporary directory within `.tmp/` at the project root
///
/// This ensures all test temporary files are centralized in a single location
/// that is gitignored and easy to clean up manually if needed.
///
/// # Returns
///
/// A `TempDir` instance that automatically cleans up on drop.
/// The directory is created at `.tmp/<random-name>` relative to the project root.
///
/// # Panics
///
/// Panics if:
/// - Unable to determine current directory
/// - Unable to create `.tmp/` directory
/// - Unable to create temporary subdirectory
///
/// # Examples
///
/// ```rust
/// use typstlab_testkit::temp_dir_in_workspace;
///
/// let temp = temp_dir_in_workspace();
/// let file_path = temp.path().join("test.txt");
/// std::fs::write(&file_path, "test data").unwrap();
/// // Cleanup happens automatically when temp is dropped
/// ```
pub fn temp_dir_in_workspace() -> TempDir {
    let workspace_root = std::env::current_dir().expect("Failed to get current directory");

    let tmp_base = workspace_root.join(".tmp");

    // Ensure .tmp/ exists
    std::fs::create_dir_all(&tmp_base).expect("Failed to create .tmp directory");

    // Create unique subdirectory within .tmp/
    TempDir::new_in(&tmp_base).expect("Failed to create temporary directory in .tmp/")
}

/// Alternative with Result for non-test code
///
/// Use this variant when you need proper error handling instead of panics.
pub fn try_temp_dir_in_workspace() -> std::io::Result<TempDir> {
    let workspace_root = std::env::current_dir()?;
    let tmp_base = workspace_root.join(".tmp");
    std::fs::create_dir_all(&tmp_base)?;
    TempDir::new_in(&tmp_base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temp_dir_in_workspace_creates_in_tmp() {
        let temp = temp_dir_in_workspace();
        let path = temp.path();

        // Verify path contains .tmp
        assert!(
            path.to_string_lossy().contains(".tmp"),
            "Path should contain .tmp, got: {}",
            path.display()
        );

        // Verify directory exists
        assert!(path.exists(), "Directory should exist");
        assert!(path.is_dir(), "Path should be a directory");
    }

    #[test]
    fn test_temp_dir_auto_cleanup() {
        let path = {
            let temp = temp_dir_in_workspace();
            let p = temp.path().to_path_buf();
            assert!(p.exists(), "Directory should exist before drop");
            p
        }; // temp dropped here

        // Directory should be cleaned up
        assert!(
            !path.exists(),
            "Directory should not exist after drop: {}",
            path.display()
        );
    }

    #[test]
    fn test_multiple_temp_dirs_unique() {
        let temp1 = temp_dir_in_workspace();
        let temp2 = temp_dir_in_workspace();

        // Should have different paths
        assert_ne!(
            temp1.path(),
            temp2.path(),
            "Multiple temp directories should have unique paths"
        );
    }

    #[test]
    fn test_try_temp_dir_in_workspace_returns_ok() {
        let result = try_temp_dir_in_workspace();
        assert!(result.is_ok(), "Should successfully create temp directory");

        let temp = result.unwrap();
        assert!(temp.path().exists());
        assert!(temp.path().to_string_lossy().contains(".tmp"));
    }
}
