//! Cross-platform path validation utilities
//!
//! This module provides platform-safe abstractions over `std::path::Path`
//! to ensure consistent behavior across Unix and Windows.
//!
//! ## The Platform Semantics Problem
//!
//! Rust's `std::path::Path::is_absolute()` has hidden platform-dependent behavior:
//!
//! - Unix: `Path::new("/tmp").is_absolute()` → `true`
//! - Windows: `Path::new("/tmp").is_absolute()` → `false` (rooted, not absolute!)
//!
//! This causes security checks to pass on macOS but fail silently on Windows.
//!
//! ## Solution
//!
//! Use component-based analysis instead of relying on `is_absolute()`:
//!
//! ```rust
//! use typstlab_core::path::has_absolute_or_rooted_component;
//! use std::path::Path;
//!
//! // ✅ Good: Cross-platform
//! if has_absolute_or_rooted_component(Path::new("/tmp")) {
//!     eprintln!("Path cannot be absolute or rooted");
//! }
//!
//! // ❌ Bad: Platform-dependent
//! if Path::new("/tmp").is_absolute() {
//!     eprintln!("Path cannot be absolute");
//! }
//! ```

use anyhow::{bail, Result};
use std::path::{Component, Path};

/// Check if path is absolute OR rooted (cross-platform)
///
/// Unlike `Path::is_absolute()`, this treats both Unix absolute paths
/// (`/tmp`) and Windows rooted paths (`/tmp`) as requiring rejection.
///
/// # Platform Differences
///
/// - Unix: `/tmp` → is_absolute() = true
/// - Windows: `/tmp` → is_absolute() = false (rooted, not absolute!)
/// - Windows: `C:\tmp` → is_absolute() = true
///
/// This function uses component-based analysis for consistent behavior.
///
/// # Examples
///
/// ```rust
/// use std::path::Path;
/// use typstlab_core::path::has_absolute_or_rooted_component;
///
/// // Unix absolute path / Windows rooted path (testable on all platforms)
/// assert!(has_absolute_or_rooted_component(Path::new("/tmp")));
///
/// // Nested path with root component (testable on all platforms)
/// assert!(has_absolute_or_rooted_component(Path::new("/etc/passwd")));
///
/// // Relative path
/// assert!(!has_absolute_or_rooted_component(Path::new("foo/bar")));
///
/// // Single component (relative)
/// assert!(!has_absolute_or_rooted_component(Path::new("my-project")));
/// ```
///
/// **Note**: Windows absolute paths with drive letters (e.g., `C:\tmp`)
/// are only testable on Windows due to platform-specific semantics.
pub fn has_absolute_or_rooted_component(path: &Path) -> bool {
    // Fast path: Platform-specific absolute check
    if path.is_absolute() {
        return true;
    }

    // Slow path: Check for rooted paths (Windows /tmp case)
    path.components()
        .any(|c| matches!(c, Component::RootDir | Component::Prefix(_)))
}

/// Check if path is safe for use as a single directory name
///
/// Validates that the path:
/// 1. Is not absolute or rooted
/// 2. Contains no parent directory traversal (..)
/// 3. Contains no current directory components (.)
/// 4. Contains exactly one Normal component (no separators)
///
/// # Errors
///
/// Returns `PathSecurityError` describing the validation failure.
///
/// # Examples
///
/// ```rust
/// use std::path::Path;
/// use typstlab_core::path::is_safe_single_component;
///
/// // Valid single component
/// assert!(is_safe_single_component(Path::new("my-project")).is_ok());
///
/// // Invalid: absolute path
/// assert!(is_safe_single_component(Path::new("/tmp")).is_err());
///
/// // Invalid: parent directory traversal
/// assert!(is_safe_single_component(Path::new("../etc")).is_err());
///
/// // Invalid: multiple components
/// assert!(is_safe_single_component(Path::new("foo/bar")).is_err());
/// ```
pub fn is_safe_single_component(path: &Path) -> Result<()> {
    // Check for absolute or rooted paths first
    if has_absolute_or_rooted_component(path) {
        bail!("Path cannot be absolute or rooted: '{}'", path.display());
    }

    let mut normal_count = 0;

    for component in path.components() {
        match component {
            Component::Normal(_) => normal_count += 1,
            Component::Prefix(_) => {
                bail!("Path cannot contain drive prefix: '{}'", path.display())
            }
            Component::RootDir => {
                bail!("Path cannot be absolute or rooted: '{}'", path.display())
            }
            Component::CurDir => {
                bail!(
                    "Path cannot contain current directory (.): '{}'",
                    path.display()
                )
            }
            Component::ParentDir => {
                bail!(
                    "Path cannot contain parent directory (..): '{}'",
                    path.display()
                )
            }
        }
    }

    // Must be exactly one component (no separators)
    if normal_count != 1 {
        bail!(
            "Path must be a single component, found {}: '{}'",
            normal_count,
            path.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Tests for has_absolute_or_rooted_component()
    // ============================================================================

    #[test]
    fn test_unix_absolute_detected() {
        let path = Path::new("/tmp");
        assert!(
            has_absolute_or_rooted_component(path),
            "Unix absolute path should be detected"
        );
    }

    #[test]
    fn test_unix_absolute_nested_detected() {
        let path = Path::new("/etc/passwd");
        assert!(
            has_absolute_or_rooted_component(path),
            "Unix absolute nested path should be detected"
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_absolute_with_drive_detected() {
        // This test only runs on Windows where C:\ is recognized as absolute
        let path = Path::new("C:\\Windows");
        assert!(
            has_absolute_or_rooted_component(path),
            "Windows absolute path with drive should be detected"
        );
    }

    #[test]
    fn test_windows_rooted_detected_on_all_platforms() {
        // THE CRITICAL TEST: Must pass on macOS to catch Windows bugs
        let path = Path::new("/tmp");

        // Verify Component structure (universal across platforms)
        let components: Vec<Component> = path.components().collect();
        assert!(
            matches!(components.first(), Some(Component::RootDir)),
            "Path /tmp should have RootDir component"
        );

        // Verify abstraction catches it (universal)
        assert!(
            has_absolute_or_rooted_component(path),
            "Windows rooted path /tmp should be detected on all platforms"
        );
    }

    #[test]
    fn test_relative_path_not_detected() {
        let path = Path::new("foo/bar");
        assert!(
            !has_absolute_or_rooted_component(path),
            "Relative path should not be detected"
        );
    }

    #[test]
    fn test_single_component_not_detected() {
        let path = Path::new("my-project");
        assert!(
            !has_absolute_or_rooted_component(path),
            "Single component should not be detected"
        );
    }

    #[test]
    fn test_current_dir_not_detected() {
        let path = Path::new("./foo");
        assert!(
            !has_absolute_or_rooted_component(path),
            "Current directory path should not be detected"
        );
    }

    #[test]
    fn test_parent_dir_not_detected() {
        let path = Path::new("../foo");
        assert!(
            !has_absolute_or_rooted_component(path),
            "Parent directory path should not be detected by this function"
        );
    }

    // ============================================================================
    // Tests for is_safe_single_component()
    // ============================================================================

    #[test]
    fn test_valid_single_component() {
        let result = is_safe_single_component(Path::new("my-project"));
        assert!(result.is_ok(), "Valid single component should be accepted");
    }

    #[test]
    fn test_valid_single_component_with_special_chars() {
        let result = is_safe_single_component(Path::new("my_project-v1.0"));
        assert!(
            result.is_ok(),
            "Valid single component with special chars should be accepted"
        );
    }

    #[test]
    fn test_unix_absolute_rejected() {
        let result = is_safe_single_component(Path::new("/tmp"));
        assert!(result.is_err(), "Unix absolute path should be rejected");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("absolute or rooted"),
            "Error should mention 'absolute or rooted', got: {}",
            err_msg
        );
    }

    #[test]
    fn test_windows_rooted_rejected() {
        // This test verifies Windows rooted paths are caught on all platforms
        let result = is_safe_single_component(Path::new("/tmp/malicious"));
        assert!(
            result.is_err(),
            "Windows rooted path should be rejected on all platforms"
        );

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("absolute or rooted"),
            "Error should mention 'absolute or rooted', got: {}",
            err_msg
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_absolute_with_drive_rejected() {
        // This test only runs on Windows where C:\ is recognized as absolute
        let result = is_safe_single_component(Path::new("C:\\malicious"));
        assert!(
            result.is_err(),
            "Windows absolute path with drive should be rejected"
        );
    }

    #[test]
    fn test_parent_directory_traversal_rejected() {
        let result = is_safe_single_component(Path::new("../etc"));
        assert!(
            result.is_err(),
            "Parent directory traversal should be rejected"
        );

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("parent directory"),
            "Error should mention 'parent directory', got: {}",
            err_msg
        );
    }

    #[test]
    fn test_current_directory_rejected() {
        let result = is_safe_single_component(Path::new("./foo"));
        assert!(
            result.is_err(),
            "Current directory component should be rejected"
        );

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("current directory"),
            "Error should mention 'current directory', got: {}",
            err_msg
        );
    }

    #[test]
    fn test_multiple_components_rejected() {
        let result = is_safe_single_component(Path::new("foo/bar"));
        assert!(result.is_err(), "Multiple components should be rejected");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("single component"),
            "Error should mention 'single component', got: {}",
            err_msg
        );
    }

    #[test]
    fn test_empty_path_rejected() {
        let result = is_safe_single_component(Path::new(""));
        assert!(result.is_err(), "Empty path should be rejected");
    }

    // ============================================================================
    // Dangerous Path Matrix Test (Cross-Platform Coverage)
    // ============================================================================

    #[test]
    fn test_dangerous_path_matrix() {
        // Test cases that work on all platforms (Unix absolute + Windows rooted)
        let cross_platform_cases = vec![
            ("/tmp", true, "Unix absolute / Windows rooted"),
            (
                "/etc/passwd",
                true,
                "Unix absolute nested / Windows rooted nested",
            ),
            ("/tmp/malicious", true, "Windows rooted (critical!)"),
            ("foo/bar", false, "Relative multi-component"),
            ("my-project", false, "Single component"),
            ("./foo", false, "Current directory"),
            ("../foo", false, "Parent directory (not absolute)"),
        ];

        for (path_str, should_be_absolute, description) in cross_platform_cases {
            let path = Path::new(path_str);
            let result = has_absolute_or_rooted_component(path);
            assert_eq!(
                result, should_be_absolute,
                "Path '{}' ({}): expected has_absolute_or_rooted_component={}, got={}",
                path_str, description, should_be_absolute, result
            );
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_dangerous_path_matrix_windows_only() {
        // Test cases that only work on Windows (drive letters and UNC paths)
        let windows_only_cases = vec![
            ("C:\\Windows", true, "Windows absolute with drive"),
            ("D:\\malicious", true, "Windows absolute with D: drive"),
            ("\\\\server\\share", true, "UNC path (Windows)"),
        ];

        for (path_str, should_be_absolute, description) in windows_only_cases {
            let path = Path::new(path_str);
            let result = has_absolute_or_rooted_component(path);
            assert_eq!(
                result, should_be_absolute,
                "Path '{}' ({}): expected has_absolute_or_rooted_component={}, got={}",
                path_str, description, should_be_absolute, result
            );
        }
    }

    // ============================================================================
    // Component-Based Testing (Verifies Windows Behavior on macOS)
    // ============================================================================

    #[test]
    fn test_component_structure_windows_rooted() {
        // This test verifies we can detect Windows rooted paths by analyzing
        // components, even when running on macOS
        let path = Path::new("/tmp");
        let components: Vec<Component> = path.components().collect();

        // On BOTH Unix and Windows, /tmp starts with RootDir
        assert!(
            !components.is_empty(),
            "Path should have at least one component"
        );
        assert!(
            matches!(components[0], Component::RootDir),
            "First component should be RootDir"
        );

        // Therefore, our abstraction should catch it on BOTH platforms
        assert!(
            has_absolute_or_rooted_component(path),
            "Abstraction should detect rooted path via components"
        );
    }

    #[test]
    fn test_component_structure_relative() {
        let path = Path::new("foo/bar");
        let components: Vec<Component> = path.components().collect();

        // Should only have Normal components
        for component in components {
            assert!(
                matches!(component, Component::Normal(_)),
                "Relative path should only have Normal components"
            );
        }

        // Therefore, our abstraction should NOT detect it
        assert!(
            !has_absolute_or_rooted_component(path),
            "Abstraction should not detect relative path"
        );
    }
}
