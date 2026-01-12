//! Tests for project detection (Project::find_root)

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use typstlab_core::project::Project;

/// Helper: Create a temporary project with typstlab.toml
fn create_temp_project() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("typstlab.toml");

    // Create minimal valid typstlab.toml
    let minimal_config = r#"
[project]
name = "test-project"
init_date = "2026-01-12"

[typst]
version = "0.17.0"
"#;

    fs::write(&config_path, minimal_config).expect("Failed to write config");
    let root = temp_dir.path().to_path_buf();

    (temp_dir, root)
}

#[test]
fn test_find_root_in_current_dir() {
    // Arrange: Create project with typstlab.toml in root
    let (_temp, project_root) = create_temp_project();

    // Act: Find root from current directory
    let result = Project::find_root(&project_root);

    // Assert: Should find project
    assert!(result.is_ok(), "find_root should succeed");
    let project = result.unwrap();
    assert!(project.is_some(), "Should find project");

    let project = project.unwrap();
    assert_eq!(
        project.root.canonicalize().unwrap(),
        project_root.canonicalize().unwrap(),
        "Root should match project root"
    );
}

#[test]
fn test_find_root_in_parent_dir() {
    // Arrange: Create project with subdirectory
    let (_temp, project_root) = create_temp_project();
    let subdir = project_root.join("papers").join("paper1");
    fs::create_dir_all(&subdir).expect("Failed to create subdirectory");

    // Act: Find root from subdirectory
    let result = Project::find_root(&subdir);

    // Assert: Should find project in parent
    assert!(result.is_ok(), "find_root should succeed");
    let project = result.unwrap();
    assert!(project.is_some(), "Should find project in parent");

    let project = project.unwrap();
    assert_eq!(
        project.root.canonicalize().unwrap(),
        project_root.canonicalize().unwrap(),
        "Root should match project root, not subdirectory"
    );
}

#[test]
fn test_find_root_not_found() {
    // Arrange: Create temp directory WITHOUT typstlab.toml
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let non_project_dir = temp_dir.path().to_path_buf();

    // Act: Find root from non-project directory
    let result = Project::find_root(&non_project_dir);

    // Assert: Should return None (not found)
    assert!(result.is_ok(), "find_root should not error");
    let project = result.unwrap();
    assert!(
        project.is_none(),
        "Should not find project in non-project directory"
    );
}

#[test]
fn test_find_root_stops_at_filesystem_root() {
    // Arrange: Create temp directory structure deep in filesystem
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let deep_dir = temp_dir.path().join("a").join("b").join("c");
    fs::create_dir_all(&deep_dir).expect("Failed to create deep directory");

    // Act: Find root from deep directory (no typstlab.toml anywhere)
    let result = Project::find_root(&deep_dir);

    // Assert: Should return None and not panic at filesystem root
    assert!(
        result.is_ok(),
        "find_root should handle filesystem root gracefully"
    );
    let project = result.unwrap();
    assert!(
        project.is_none(),
        "Should not find project and should stop at filesystem root"
    );
}

#[test]
fn test_find_root_multiple_levels_up() {
    // Arrange: Create project with deeply nested subdirectory
    let (_temp, project_root) = create_temp_project();
    let deep_subdir = project_root
        .join("papers")
        .join("paper1")
        .join("sections")
        .join("nested");
    fs::create_dir_all(&deep_subdir).expect("Failed to create deep subdirectory");

    // Act: Find root from deeply nested directory
    let result = Project::find_root(&deep_subdir);

    // Assert: Should traverse multiple levels to find project
    assert!(result.is_ok(), "find_root should succeed");
    let project = result.unwrap();
    assert!(project.is_some(), "Should find project multiple levels up");

    let project = project.unwrap();
    assert_eq!(
        project.root.canonicalize().unwrap(),
        project_root.canonicalize().unwrap(),
        "Root should match project root after traversing multiple levels"
    );
}

#[test]
fn test_find_root_with_symlink() {
    // Arrange: Create project and symlink to subdirectory
    let (_temp, project_root) = create_temp_project();
    let real_dir = project_root.join("papers");
    fs::create_dir_all(&real_dir).expect("Failed to create directory");

    // Create symlink (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let link_path = project_root.join("papers_link");
        symlink(&real_dir, &link_path).expect("Failed to create symlink");

        // Act: Find root from symlinked directory
        let result = Project::find_root(&link_path);

        // Assert: Should resolve symlink and find project
        assert!(result.is_ok(), "find_root should handle symlinks");
        let project = result.unwrap();
        assert!(project.is_some(), "Should find project through symlink");

        let project = project.unwrap();
        // After canonicalize, symlink is resolved
        let canonical_root = project.root.canonicalize().unwrap();
        let expected_root = project_root.canonicalize().unwrap();
        assert_eq!(
            canonical_root, expected_root,
            "Root should match project root even through symlink"
        );
    }

    // On Windows, this test is skipped
    #[cfg(windows)]
    {
        println!("SKIPPED: Symlink test requires admin privileges on Windows");
        println!("Cross-platform path handling is already tested in other tests");
    }
}
