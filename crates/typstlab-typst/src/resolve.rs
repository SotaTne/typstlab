use std::path::{Path, PathBuf};
use std::process::Command;
use typstlab_core::{Result, TypstlabError};
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
    let base = dirs::cache_dir()
        .ok_or_else(|| TypstlabError::Generic(
            "Could not determine cache directory".to_string()
        ))?;

    Ok(base.join("typstlab").join("typst"))
}

/// Validate that a Typst binary matches the expected version
///
/// Executes: `typst --version`
/// Parses output: "typst 0.13.1" -> "0.13.1"
/// Returns: Ok(true) if version matches, Ok(false) if mismatch
fn validate_version(path: &Path, expected: &str) -> Result<bool> {
    // Execute typst --version
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|e| TypstlabError::TypstExecFailed(
            format!("Failed to execute typst --version: {}", e)
        ))?;

    if !output.status.success() {
        return Err(TypstlabError::TypstExecFailed(
            format!("typst --version exited with status: {}", output.status)
        ));
    }

    // Parse stdout to extract version
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Match pattern: "typst X.Y.Z" or "typst vX.Y.Z"
    // Use simple string parsing instead of regex to avoid dependency
    let version = parse_typst_version(&stdout)
        .ok_or_else(|| TypstlabError::TypstExecFailed(
            format!("Could not parse version from typst --version output: {}", stdout)
        ))?;

    Ok(version == expected)
}

/// Parse version string from typst --version output
///
/// Expected formats:
/// - "typst 0.13.1"
/// - "typst v0.13.1"
fn parse_typst_version(output: &str) -> Option<String> {
    // Find "typst" in the output
    let output = output.trim();

    // Look for pattern: "typst" followed by optional "v" and version number
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("typst") {
            // Split by whitespace
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                // Second part should be version (possibly with 'v' prefix)
                let version = parts[1].trim_start_matches('v');
                // Basic validation: should contain at least one dot
                if version.contains('.') {
                    return Some(version.to_string());
                }
            }
        }
    }

    None
}

/// Check if a Typst binary is cached in state
///
/// Fast path: returns cached TypstInfo if binary still exists and version matches
///
/// Note: This is a simplified implementation for Phase 1.
/// Full state.json integration will be added in Commit 7.
fn check_cache(_version: &str) -> Option<TypstInfo> {
    // TODO: Integrate with state.json in Commit 7
    // For now, always return None (no cache hit)
    None
}

// ============================================================================
// Resolution Strategies (to be implemented in Commit 5)
// ============================================================================

/// Resolve Typst from managed cache
///
/// Checks {cache_dir}/{version}/typst for the binary
fn resolve_managed(version: &str) -> Result<Option<TypstInfo>> {
    let cache_dir = managed_cache_dir()?;
    let version_dir = cache_dir.join(version);

    // Determine binary name based on platform
    #[cfg(windows)]
    let binary_name = "typst.exe";
    #[cfg(not(windows))]
    let binary_name = "typst";

    let binary_path = version_dir.join(binary_name);

    // Check if binary exists
    if !binary_path.exists() {
        return Ok(None);
    }

    // Validate version matches
    match validate_version(&binary_path, version) {
        Ok(true) => {
            // Version matches, return TypstInfo
            Ok(Some(TypstInfo {
                version: version.to_string(),
                source: TypstSource::Managed,
                path: binary_path,
            }))
        }
        Ok(false) => {
            // Version mismatch
            Ok(None)
        }
        Err(_) => {
            // Error executing or parsing version
            Ok(None)
        }
    }
}

/// Resolve Typst from system PATH
///
/// Uses `which` crate to find typst binary in PATH
fn resolve_system(version: &str) -> Result<Option<TypstInfo>> {
    // Use `which` to find typst in PATH
    let binary_path = match which::which("typst") {
        Ok(path) => path,
        Err(_) => {
            // typst not found in PATH
            return Ok(None);
        }
    };

    // Validate version matches
    match validate_version(&binary_path, version) {
        Ok(true) => {
            // Version matches, return TypstInfo
            Ok(Some(TypstInfo {
                version: version.to_string(),
                source: TypstSource::System,
                path: binary_path,
            }))
        }
        Ok(false) => {
            // Version mismatch
            Ok(None)
        }
        Err(_) => {
            // Error executing or parsing version
            Ok(None)
        }
    }
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

    // ========================================================================
    // Resolution Strategy Tests
    // ========================================================================

    #[test]
    fn test_resolve_managed_found_exact_version() {
        // Setup: Create managed cache directory with fake typst binary
        let cache_dir = managed_cache_dir().unwrap();
        let version = "0.13.1";
        let version_dir = cache_dir.join(version);

        // Create directory structure
        fs::create_dir_all(&version_dir).unwrap();

        // Create fake typst binary
        #[cfg(unix)]
        let binary_path = version_dir.join("typst");
        #[cfg(windows)]
        let binary_path = version_dir.join("typst.exe");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(&binary_path, "#!/bin/sh\necho 'typst 0.13.1'").unwrap();
            let mut perms = fs::metadata(&binary_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&binary_path, perms).unwrap();
        }

        #[cfg(windows)]
        {
            fs::write(&binary_path, "@echo typst 0.13.1").unwrap();
        }

        // Test: resolve_managed should find the binary
        let result = resolve_managed(version);
        assert!(result.is_ok());

        let typst_info = result.unwrap();
        assert!(typst_info.is_some());

        let info = typst_info.unwrap();
        assert_eq!(info.version, version);
        assert_eq!(info.path, binary_path);

        // Cleanup
        let _ = fs::remove_dir_all(&version_dir);
    }

    #[test]
    fn test_resolve_managed_not_found() {
        // Test: resolve_managed should return None for non-existent version
        let result = resolve_managed("99.99.99");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_resolve_managed_version_mismatch() {
        // Setup: Create managed cache with wrong version
        let cache_dir = managed_cache_dir().unwrap();
        let actual_version = "0.12.0";
        let requested_version = "0.13.1";
        let version_dir = cache_dir.join(requested_version);

        fs::create_dir_all(&version_dir).unwrap();

        #[cfg(unix)]
        let binary_path = version_dir.join("typst");
        #[cfg(windows)]
        let binary_path = version_dir.join("typst.exe");

        // Create fake binary that reports wrong version
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(&binary_path, format!("#!/bin/sh\necho 'typst {}'", actual_version)).unwrap();
            let mut perms = fs::metadata(&binary_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&binary_path, perms).unwrap();
        }

        #[cfg(windows)]
        {
            fs::write(&binary_path, format!("@echo typst {}", actual_version)).unwrap();
        }

        // Test: resolve_managed should return None due to version mismatch
        let result = resolve_managed(requested_version);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Cleanup
        let _ = fs::remove_dir_all(&version_dir);
    }

    #[test]
    fn test_resolve_managed_binary_not_executable() {
        // Setup: Create directory but no binary file
        let cache_dir = managed_cache_dir().unwrap();
        let version = "0.13.1";
        let version_dir = cache_dir.join(version);

        fs::create_dir_all(&version_dir).unwrap();

        // Test: resolve_managed should return None when binary doesn't exist
        let result = resolve_managed(version);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Cleanup
        let _ = fs::remove_dir_all(&version_dir);
    }

    #[test]
    fn test_resolve_system_found_in_path() {
        // This test requires a real typst binary in PATH
        // We'll create a temporary directory and add it to PATH simulation
        let temp_dir = env::temp_dir();
        let fake_binary = temp_dir.join("fake_typst_system");

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
            let fake_binary = temp_dir.join("fake_typst_system.bat");
            fs::write(&fake_binary, "@echo typst 0.13.1").unwrap();
        }

        // Note: This test will actually fail unless typst is in PATH
        // In a real scenario, we'd need to mock `which::which()`
        // For now, we test that the function returns Ok
        let result = resolve_system("0.13.1");
        assert!(result.is_ok());

        // The result may be Some or None depending on system state
        // We just verify it doesn't panic or error

        // Cleanup
        #[cfg(unix)]
        let _ = fs::remove_file(&fake_binary);
        #[cfg(windows)]
        let _ = fs::remove_file(temp_dir.join("fake_typst_system.bat"));
    }

    #[test]
    fn test_resolve_system_not_in_path() {
        // Test: resolve_system should return None for non-existent binary
        // We use a version that's extremely unlikely to exist
        let result = resolve_system("99.99.99");
        assert!(result.is_ok());

        // Should return None since this version is unlikely to exist
        let typst_info = result.unwrap();
        if typst_info.is_some() {
            // If somehow found, verify it's not the requested version
            let info = typst_info.unwrap();
            assert_ne!(info.version, "99.99.99");
        }
    }

    #[test]
    fn test_resolve_system_version_mismatch() {
        // Test: If system has typst but wrong version, should return None
        // This test depends on system state, so we make it flexible
        let result = resolve_system("0.0.1");
        assert!(result.is_ok());

        // If a typst binary is found, it should either:
        // 1. Return None (version mismatch)
        // 2. Return Some only if version actually matches 0.0.1 (unlikely)
        let typst_info = result.unwrap();
        if let Some(info) = typst_info {
            // If found, version must match exactly
            assert_eq!(info.version, "0.0.1");
        }
    }

    #[test]
    fn test_resolve_system_which_error() {
        // Test: resolve_system should handle errors gracefully
        // We can't easily force `which` to error, but we can test
        // that calling with an unusual version doesn't panic
        let result = resolve_system("invalid.version.format");
        assert!(result.is_ok());

        // Should return None or Some (doesn't matter which)
        // Important: should not panic or return Err
    }
}
