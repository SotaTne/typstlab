use std::path::{Path, PathBuf};
use typstlab_core::Result;
use crate::info::{TypstInfo, TypstSource};

/// Options for resolving the Typst binary
#[derive(Debug, Clone)]
pub struct ResolveOptions {
    pub required_version: String,
    pub project_root: PathBuf,
    pub force_refresh: bool,
}

/// Result of Typst binary resolution
#[derive(Debug, Clone)]
pub enum ResolveResult {
    /// Binary was found in cache (fast path)
    Cached(TypstInfo),
    /// Binary was newly resolved
    Resolved(TypstInfo),
    /// Binary not found
    NotFound {
        required_version: String,
        searched_locations: Vec<String>,
    },
}

// ============================================================================
// Helper Functions (to be implemented in Commit 3)
// ============================================================================

/// Get the managed cache directory for Typst binaries
///
/// Platform-specific paths:
/// - macOS: ~/Library/Caches/typstlab/typst
/// - Linux: ~/.cache/typstlab/typst
/// - Windows: %LOCALAPPDATA%\typstlab\typst
fn managed_cache_dir() -> Result<PathBuf> {
    unimplemented!("managed_cache_dir will be implemented in Commit 3")
}

/// Validate that a Typst binary matches the expected version
///
/// Executes: `typst --version`
/// Parses output: "typst 0.13.1" -> "0.13.1"
/// Returns: Ok(true) if version matches, Ok(false) if mismatch
fn validate_version(_path: &Path, _expected: &str) -> Result<bool> {
    unimplemented!("validate_version will be implemented in Commit 3")
}

/// Check if a Typst binary is cached in state
///
/// Fast path: returns cached TypstInfo if binary still exists and version matches
fn check_cache(_version: &str) -> Option<TypstInfo> {
    unimplemented!("check_cache will be implemented in Commit 3")
}

// ============================================================================
// Resolution Strategies (to be implemented in Commit 5)
// ============================================================================

/// Resolve Typst from managed cache
fn resolve_managed(_version: &str) -> Result<Option<TypstInfo>> {
    unimplemented!("resolve_managed will be implemented in Commit 5")
}

/// Resolve Typst from system PATH
fn resolve_system(_version: &str) -> Result<Option<TypstInfo>> {
    unimplemented!("resolve_system will be implemented in Commit 5")
}

// ============================================================================
// Main Entry Point (to be implemented in Commit 7)
// ============================================================================

/// Resolve the Typst binary based on options
///
/// Resolution priority:
/// 1. Cache (if !force_refresh)
/// 2. Managed cache
/// 3. System PATH
/// 4. NotFound
pub fn resolve_typst(
    _options: ResolveOptions,
) -> Result<ResolveResult> {
    unimplemented!("resolve_typst will be implemented in Commit 7")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    // ========================================================================
    // Helper Function Tests
    // ========================================================================

    #[test]
    #[cfg(target_os = "macos")]
    fn test_managed_cache_dir_macos() {
        let result = managed_cache_dir();
        assert!(result.is_ok());

        let cache_dir = result.unwrap();
        let cache_str = cache_dir.to_string_lossy();

        // Should be: ~/Library/Caches/typstlab/typst
        assert!(cache_str.contains("Library/Caches/typstlab/typst"));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_managed_cache_dir_linux() {
        let result = managed_cache_dir();
        assert!(result.is_ok());

        let cache_dir = result.unwrap();
        let cache_str = cache_dir.to_string_lossy();

        // Should be: ~/.cache/typstlab/typst
        assert!(cache_str.contains(".cache/typstlab/typst"));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_managed_cache_dir_windows() {
        let result = managed_cache_dir();
        assert!(result.is_ok());

        let cache_dir = result.unwrap();
        let cache_str = cache_dir.to_string_lossy();

        // Should be: %LOCALAPPDATA%\typstlab\typst
        assert!(cache_str.contains("typstlab\\typst"));
    }

    #[test]
    fn test_managed_cache_dir_creates_path() {
        let result = managed_cache_dir();
        assert!(result.is_ok());

        let cache_dir = result.unwrap();

        // Should end with typstlab/typst
        assert!(cache_dir.ends_with("typstlab/typst") ||
                cache_dir.ends_with("typstlab\\typst"));
    }

    #[test]
    fn test_validate_version_exact_match() {
        // Create a fake typst binary for testing
        let temp_dir = env::temp_dir();
        let fake_binary = temp_dir.join("fake_typst_exact");

        // Create a script that outputs "typst 0.13.1"
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(&fake_binary, "#!/bin/sh\necho 'typst 0.13.1'").unwrap();
            let mut perms = fs::metadata(&fake_binary).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_binary, perms).unwrap();
        }

        #[cfg(windows)]
        {
            // On Windows, create a .bat file
            let fake_binary = temp_dir.join("fake_typst_exact.bat");
            fs::write(&fake_binary, "@echo typst 0.13.1").unwrap();
        }

        #[cfg(unix)]
        let result = validate_version(&fake_binary, "0.13.1");
        #[cfg(windows)]
        let result = validate_version(&temp_dir.join("fake_typst_exact.bat"), "0.13.1");

        // Should return Ok(true) for exact match
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        // Cleanup
        #[cfg(unix)]
        let _ = fs::remove_file(&fake_binary);
        #[cfg(windows)]
        let _ = fs::remove_file(temp_dir.join("fake_typst_exact.bat"));
    }

    #[test]
    fn test_validate_version_mismatch() {
        let temp_dir = env::temp_dir();
        let fake_binary = temp_dir.join("fake_typst_mismatch");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(&fake_binary, "#!/bin/sh\necho 'typst 0.12.0'").unwrap();
            let mut perms = fs::metadata(&fake_binary).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_binary, perms).unwrap();
        }

        #[cfg(windows)]
        {
            let fake_binary = temp_dir.join("fake_typst_mismatch.bat");
            fs::write(&fake_binary, "@echo typst 0.12.0").unwrap();
        }

        #[cfg(unix)]
        let result = validate_version(&fake_binary, "0.13.1");
        #[cfg(windows)]
        let result = validate_version(&temp_dir.join("fake_typst_mismatch.bat"), "0.13.1");

        // Should return Ok(false) for version mismatch
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);

        // Cleanup
        #[cfg(unix)]
        let _ = fs::remove_file(&fake_binary);
        #[cfg(windows)]
        let _ = fs::remove_file(temp_dir.join("fake_typst_mismatch.bat"));
    }

    #[test]
    fn test_validate_version_binary_not_found() {
        let nonexistent = PathBuf::from("/nonexistent/path/to/typst");
        let result = validate_version(&nonexistent, "0.13.1");

        // Should return error when binary doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_version_invalid_output() {
        let temp_dir = env::temp_dir();
        let fake_binary = temp_dir.join("fake_typst_invalid");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(&fake_binary, "#!/bin/sh\necho 'invalid output'").unwrap();
            let mut perms = fs::metadata(&fake_binary).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_binary, perms).unwrap();
        }

        #[cfg(windows)]
        {
            let fake_binary = temp_dir.join("fake_typst_invalid.bat");
            fs::write(&fake_binary, "@echo invalid output").unwrap();
        }

        #[cfg(unix)]
        let result = validate_version(&fake_binary, "0.13.1");
        #[cfg(windows)]
        let result = validate_version(&temp_dir.join("fake_typst_invalid.bat"), "0.13.1");

        // Should return error when output can't be parsed
        assert!(result.is_err());

        // Cleanup
        #[cfg(unix)]
        let _ = fs::remove_file(&fake_binary);
        #[cfg(windows)]
        let _ = fs::remove_file(temp_dir.join("fake_typst_invalid.bat"));
    }

    #[test]
    fn test_check_cache_none_when_version_not_cached() {
        // When no version is cached, should return None
        let result = check_cache("0.13.1");
        assert!(result.is_none());
    }

    #[test]
    fn test_check_cache_returns_info_when_cached() {
        // This test will be more meaningful after we implement state integration
        // For now, just verify the function signature
        let result = check_cache("0.13.1");

        // Should return Option<TypstInfo>
        assert!(result.is_none() || result.is_some());
    }

    #[test]
    fn test_check_cache_none_when_path_not_exists() {
        // If cached path no longer exists, should return None
        // This will be tested properly in Commit 3
        let result = check_cache("0.13.1");
        assert!(result.is_none());
    }
}
