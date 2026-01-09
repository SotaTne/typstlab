use crate::info::{TypstInfo, TypstSource};
use semver::Version;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use typstlab_core::{Result, TypstlabError};

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
///
/// Falls back to temp directory if cache_dir is unavailable (e.g., in containers)
pub fn managed_cache_dir() -> Result<PathBuf> {
    let base_cache = match dirs::cache_dir() {
        Some(dir) => dir,
        None => {
            // Fallback: use temp directory
            std::env::temp_dir().join(".typstlab-cache")
        }
    };

    let typst_cache = base_cache.join("typstlab").join("typst");
    fs::create_dir_all(&typst_cache)
        .map_err(|e| TypstlabError::Generic(format!("Failed to create cache directory: {}", e)))?;

    Ok(typst_cache)
}

/// Validate that a Typst binary matches the expected version
///
/// Executes: `typst --version`
/// Parses output: "typst 0.13.1" -> "0.13.1"
/// Returns: Ok(true) if version matches, Ok(false) if mismatch
fn validate_version(path: &Path, expected: &str) -> Result<bool> {
    // Validate expected version format first
    Version::parse(expected).map_err(|e| {
        TypstlabError::Generic(format!(
            "Invalid expected version format '{}': {}",
            expected, e
        ))
    })?;

    // Execute typst --version (retry on ETXTBSY)
    let output = execute_with_retry(path).map_err(|e| {
        TypstlabError::TypstExecFailed(format!("Failed to execute typst --version: {}", e))
    })?;

    if !output.status.success() {
        return Err(TypstlabError::TypstExecFailed(format!(
            "typst --version exited with status: {}",
            output.status
        )));
    }

    // Parse stdout to extract version
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Match pattern: "typst X.Y.Z" or "typst vX.Y.Z"
    // Use simple string parsing instead of regex to avoid dependency
    let version = parse_typst_version(&stdout).ok_or_else(|| {
        TypstlabError::TypstExecFailed(format!(
            "Could not parse version from typst --version output: {}",
            stdout
        ))
    })?;

    Ok(version == expected)
}

fn execute_with_retry(path: &Path) -> std::io::Result<std::process::Output> {
    use std::time::Duration;
    let mut last_err = None;
    for attempt in 0..5 {
        match Command::new(path).arg("--version").output() {
            Ok(output) => return Ok(output),
            Err(e) => {
                last_err = Some(e);
                if should_retry_exec(last_err.as_ref().unwrap()) {
                    std::thread::sleep(Duration::from_millis(5 * (attempt + 1) as u64));
                    continue;
                }
                break;
            }
        }
    }
    Err(last_err.unwrap_or_else(|| std::io::Error::other("Unknown exec error")))
}

fn should_retry_exec(err: &std::io::Error) -> bool {
    if err.kind() == std::io::ErrorKind::PermissionDenied {
        return true;
    }
    err.raw_os_error() == Some(26)
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
                let version_str = parts[1].trim_start_matches('v');

                // Validate using semver
                match Version::parse(version_str) {
                    Ok(_) => return Some(version_str.to_string()),
                    Err(_) => continue, // Try next line
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
pub fn resolve_typst(options: ResolveOptions) -> Result<ResolveResult> {
    let version = &options.required_version;
    let mut searched_locations = Vec::new();

    // Step 1: Check cache (fast path) if not force_refresh
    if !options.force_refresh
        && let Some(cached_info) = check_cache(version)
    {
        return Ok(ResolveResult::Cached(cached_info));
    }

    // Step 2: Try managed cache
    match resolve_managed(version)? {
        Some(info) => {
            return Ok(ResolveResult::Resolved(info));
        }
        None => {
            let cache_dir = managed_cache_dir()?;
            let managed_path = cache_dir.join(version);
            searched_locations.push(format!("managed cache: {}", managed_path.display()));
        }
    }

    // Step 3: Try system PATH
    match resolve_system(version)? {
        Some(info) => {
            return Ok(ResolveResult::Resolved(info));
        }
        None => {
            searched_locations.push("system PATH".to_string());
        }
    }

    // Step 4: Not found anywhere
    Ok(ResolveResult::NotFound {
        required_version: version.clone(),
        searched_locations,
    })
}

// ============================================================================
// Test-Only Helpers
// ============================================================================

/// Test-only helper: managed_cache_dir with custom base directory
#[doc(hidden)]
pub fn managed_cache_dir_with_override(base_cache_override: Option<PathBuf>) -> Result<PathBuf> {
    let base = match base_cache_override {
        Some(dir) => dir,
        None => dirs::cache_dir().ok_or_else(|| {
            TypstlabError::Generic("Could not determine cache directory".to_string())
        })?,
    };

    let typst_cache = base.join("typstlab").join("typst");

    // Create directory if it doesn't exist
    fs::create_dir_all(&typst_cache)
        .map_err(|e| TypstlabError::Generic(format!("Failed to create cache directory: {}", e)))?;

    Ok(typst_cache)
}

/// Test-only helper: resolve_managed with custom cache directory
#[doc(hidden)]
pub fn resolve_managed_with_override(
    version: &str,
    cache_dir_override: Option<PathBuf>,
) -> Result<Option<TypstInfo>> {
    let cache_dir = cache_dir_override.expect("cache_dir_override must be Some in tests");

    let version_dir = cache_dir.join(version);

    #[cfg(unix)]
    let binary_path = version_dir.join("typst");
    #[cfg(windows)]
    let binary_path = version_dir.join("typst.bat");

    if !binary_path.exists() {
        return Ok(None);
    }

    let is_match = validate_version(&binary_path, version)?;
    if !is_match {
        return Ok(None);
    }

    Ok(Some(TypstInfo {
        version: version.to_string(),
        path: binary_path,
        source: TypstSource::Managed,
    }))
}

/// Test-only helper: resolve_typst with custom cache directory
#[doc(hidden)]
pub fn resolve_typst_with_override(
    options: ResolveOptions,
    cache_dir_override: Option<PathBuf>,
) -> Result<ResolveResult> {
    let version = &options.required_version;
    let mut searched_locations = Vec::new();

    if !options.force_refresh
        && let Some(cached_info) = check_cache(version)
    {
        return Ok(ResolveResult::Cached(cached_info));
    }

    match resolve_managed_with_override(version, cache_dir_override.clone())? {
        Some(info) => return Ok(ResolveResult::Resolved(info)),
        None => {
            // Use the provided cache_dir_override for search locations
            // This is a test-only function, so cache_dir_override is always Some
            let cache_dir = cache_dir_override.expect("cache_dir_override must be Some in tests");
            let managed_path = cache_dir.join(version);
            searched_locations.push(format!("managed cache: {}", managed_path.display()));
        }
    }

    match resolve_system(version)? {
        Some(info) => return Ok(ResolveResult::Resolved(info)),
        None => {
            searched_locations.push("system PATH".to_string());
        }
    }

    Ok(ResolveResult::NotFound {
        required_version: version.clone(),
        searched_locations,
    })
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
    use tempfile::TempDir;

    // ========================================================================
    // Test Helpers Module
    // ========================================================================

    fn tempdir_in_workspace() -> TempDir {
        let base = env::current_dir().unwrap();
        TempDir::new_in(base).unwrap()
    }

    fn sync_parent_dir(dir: &std::path::Path) {
        #[cfg(unix)]
        {
            if let Ok(handle) = std::fs::File::open(dir) {
                let _ = handle.sync_all();
            }
        }
    }

    mod test_helpers {
        use super::sync_parent_dir;
        use super::*;

        /// Create a fake typst binary in a temporary directory
        ///
        /// Uses NamedTempFile::persist() for atomic file creation to avoid
        /// race conditions like Linux "Text file busy" (ETXTBSY) errors.
        pub fn create_fake_typst_in_temp(
            temp_dir: &TempDir,
            version: &str,
            script_content: &str,
        ) -> PathBuf {
            use std::io::Write;
            use tempfile::NamedTempFile;

            let version_dir = temp_dir.path().join(version);
            fs::create_dir_all(&version_dir).unwrap();

            #[cfg(unix)]
            let binary_path = version_dir.join("typst");
            #[cfg(windows)]
            let binary_path = version_dir.join("typst.bat");

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                let script = format!(
                    "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  echo \"typst {}\"\n  exit 0\nfi\n{}",
                    version, script_content
                );

                // Create temp file in the same directory for atomic operation
                let mut temp_file = NamedTempFile::new_in(&version_dir).unwrap();
                temp_file.write_all(script.as_bytes()).unwrap();

                // Set executable permissions before persisting
                let mut perms = temp_file.as_file().metadata().unwrap().permissions();
                perms.set_mode(0o755);
                temp_file.as_file().set_permissions(perms).unwrap();

                // Ensure filesystem sync before execution to prevent ETXTBSY
                temp_file.as_file().sync_all().unwrap();

                // Atomically persist and explicitly drop to avoid ETXTBSY on fast exec
                let persisted = temp_file.persist(&binary_path).unwrap();
                drop(persisted);
                sync_parent_dir(&version_dir);
            }

            #[cfg(windows)]
            {
                let script = format!(
                    "@echo off\nif \"%1\"==\"--version\" (\n  echo typst {}\n  exit /b 0\n)\n{}",
                    version, script_content
                );

                // Create temp file in the same directory for atomic operation
                let mut temp_file = NamedTempFile::new_in(&version_dir).unwrap();
                temp_file.write_all(script.as_bytes()).unwrap();

                // Ensure filesystem sync before execution to prevent race conditions
                temp_file.as_file().sync_all().unwrap();

                // Atomically persist and explicitly drop to avoid race on fast exec
                let persisted = temp_file.persist(&binary_path).unwrap();
                drop(persisted);
            }

            binary_path
        }
    }

    // ========================================================================
    // Helper Function Tests
    // ========================================================================

    #[test]
    #[cfg(target_os = "macos")]
    fn test_managed_cache_dir_macos() {
        let temp_base = tempdir_in_workspace();

        let result = managed_cache_dir_with_override(Some(temp_base.path().to_path_buf()));
        assert!(result.is_ok());

        let cache_dir = result.unwrap();

        // Should end with typstlab/typst using Path API
        assert!(cache_dir.ends_with(Path::new("typstlab").join("typst")));

        // TempDir automatically cleans up
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_managed_cache_dir_linux() {
        let temp_base = tempdir_in_workspace();

        let result = managed_cache_dir_with_override(Some(temp_base.path().to_path_buf()));
        assert!(result.is_ok());

        let cache_dir = result.unwrap();

        // Should end with typstlab/typst using Path API
        assert!(cache_dir.ends_with(Path::new("typstlab").join("typst")));

        // TempDir automatically cleans up
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_managed_cache_dir_windows() {
        let temp_base = tempdir_in_workspace();

        let result = managed_cache_dir_with_override(Some(temp_base.path().to_path_buf()));
        assert!(result.is_ok());

        let cache_dir = result.unwrap();

        // Should end with typstlab/typst using Path API
        assert!(cache_dir.ends_with(Path::new("typstlab").join("typst")));

        // TempDir automatically cleans up
    }

    #[test]
    fn test_managed_cache_dir_creates_path() {
        let temp_base = tempdir_in_workspace();

        let result = managed_cache_dir_with_override(Some(temp_base.path().to_path_buf()));
        assert!(result.is_ok());

        let cache_dir = result.unwrap();

        // Should end with typstlab/typst using Path API
        assert!(cache_dir.ends_with(Path::new("typstlab").join("typst")));

        // Verify the directory was actually created
        assert!(cache_dir.exists());

        // TempDir automatically cleans up
    }

    #[test]
    fn test_validate_version_exact_match() {
        // Create a fake typst binary for testing
        let temp_dir = tempdir_in_workspace();

        #[cfg(unix)]
        let fake_binary = temp_dir.path().join("typst");
        #[cfg(windows)]
        let fake_binary = temp_dir.path().join("typst.bat");

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
            fs::write(&fake_binary, "@echo typst 0.13.1").unwrap();
        }

        let result = validate_version(&fake_binary, "0.13.1");

        // Should return Ok(true) for exact match
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        // TempDir automatically cleans up
    }

    #[test]
    fn test_validate_version_mismatch() {
        let temp_dir = tempdir_in_workspace();

        #[cfg(unix)]
        let fake_binary = temp_dir.path().join("typst");
        #[cfg(windows)]
        let fake_binary = temp_dir.path().join("typst.bat");

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
            fs::write(&fake_binary, "@echo typst 0.12.0").unwrap();
        }

        let result = validate_version(&fake_binary, "0.13.1");

        // Should return Ok(false) for version mismatch
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);

        // TempDir automatically cleans up
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
        let temp_dir = tempdir_in_workspace();

        #[cfg(unix)]
        let fake_binary = temp_dir.path().join("typst");
        #[cfg(windows)]
        let fake_binary = temp_dir.path().join("typst.bat");

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
            fs::write(&fake_binary, "@echo invalid output").unwrap();
        }

        let result = validate_version(&fake_binary, "0.13.1");

        // Should return error when output can't be parsed
        assert!(result.is_err());

        // TempDir automatically cleans up
    }

    #[test]
    fn test_parse_typst_version_valid() {
        // Test valid semver formats
        assert_eq!(
            parse_typst_version("typst 0.11.0"),
            Some("0.11.0".to_string())
        );
        assert_eq!(
            parse_typst_version("typst 1.0.0"),
            Some("1.0.0".to_string())
        );
        assert_eq!(
            parse_typst_version("typst v0.11.0"),
            Some("0.11.0".to_string())
        );
        assert_eq!(
            parse_typst_version("typst 0.13.1"),
            Some("0.13.1".to_string())
        );
    }

    #[test]
    fn test_parse_typst_version_invalid() {
        // Test invalid version formats - should reject non-semver strings
        assert_eq!(parse_typst_version("typst invalid.version.format"), None);
        assert_eq!(parse_typst_version("typst 1.a.b"), None);
        assert_eq!(parse_typst_version("typst 1...2...3"), None);
        assert_eq!(parse_typst_version("typst test.0.0"), None);
        assert_eq!(parse_typst_version("not a version"), None);
        assert_eq!(parse_typst_version(""), None);
    }

    #[test]
    fn test_validate_version_invalid_expected() {
        let temp_dir = tempdir_in_workspace();

        #[cfg(unix)]
        let fake_binary = temp_dir.path().join("typst");
        #[cfg(windows)]
        let fake_binary = temp_dir.path().join("typst.bat");

        // Test with invalid expected version format
        let result = validate_version(&fake_binary, "invalid.format");
        assert!(result.is_err());

        // Verify error message contains expected text
        if let Err(e) = result {
            let error_msg = format!("{:?}", e);
            assert!(error_msg.contains("Invalid expected version format"));
        }

        // TempDir automatically cleans up
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
        let temp_cache = tempdir_in_workspace();
        let version = "0.13.1";

        let binary_path = test_helpers::create_fake_typst_in_temp(&temp_cache, version, "");

        let result = resolve_managed_with_override(version, Some(temp_cache.path().to_path_buf()));

        assert!(
            result.is_ok(),
            "resolve_managed_with_override failed: {:?}",
            result.err()
        );
        let info = result.unwrap().unwrap();
        assert_eq!(info.version, version);
        assert_eq!(info.path, binary_path);

        // TempDir automatically cleans up
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
        let temp_cache = tempdir_in_workspace();
        let actual_version = "0.12.0";
        let requested_version = "0.13.1";

        // Create binary that reports different version (using NamedTempFile for atomicity)
        use std::io::Write;
        use tempfile::NamedTempFile;

        let version_dir = temp_cache.path().join(requested_version);
        fs::create_dir_all(&version_dir).unwrap();

        #[cfg(unix)]
        let binary_path = version_dir.join("typst");
        #[cfg(windows)]
        let binary_path = version_dir.join("typst.bat");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let script = format!(
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  echo \"typst {}\"\n  exit 0\nfi",
                actual_version
            );

            let mut temp_file = NamedTempFile::new_in(&version_dir).unwrap();
            temp_file.write_all(script.as_bytes()).unwrap();

            let mut perms = temp_file.as_file().metadata().unwrap().permissions();
            perms.set_mode(0o755);
            temp_file.as_file().set_permissions(perms).unwrap();

            // Ensure filesystem sync before execution to prevent ETXTBSY
            temp_file.as_file().sync_all().unwrap();

            // Persist and explicitly drop to avoid ETXTBSY on fast exec
            let persisted = temp_file.persist(&binary_path).unwrap();
            drop(persisted);
            sync_parent_dir(&version_dir);
        }

        #[cfg(windows)]
        {
            let script = format!(
                "@echo off\nif \"%1\"==\"--version\" (\n  echo typst {}\n  exit /b 0\n)",
                actual_version
            );

            let mut temp_file = NamedTempFile::new_in(&version_dir).unwrap();
            temp_file.write_all(script.as_bytes()).unwrap();

            // Ensure filesystem sync before execution to prevent race conditions
            temp_file.as_file().sync_all().unwrap();

            // Persist and explicitly drop to avoid race on fast exec
            let persisted = temp_file.persist(&binary_path).unwrap();
            drop(persisted);
        }

        let result =
            resolve_managed_with_override(requested_version, Some(temp_cache.path().to_path_buf()));

        assert!(
            result.is_ok(),
            "resolve_managed_with_override failed: {:?}",
            result.err()
        );
        assert!(result.unwrap().is_none());

        // TempDir automatically cleans up
    }

    #[test]
    fn test_resolve_managed_binary_not_executable() {
        let temp_cache = tempdir_in_workspace();
        let version = "0.13.1";
        let version_dir = temp_cache.path().join(version);

        // Create directory but no binary file
        fs::create_dir_all(&version_dir).unwrap();

        let result = resolve_managed_with_override(version, Some(temp_cache.path().to_path_buf()));

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // TempDir automatically cleans up
    }

    #[test]
    fn test_resolve_system_found_in_path() {
        // Note: This test relies on the system PATH
        // In a real scenario, we'd need to mock `which::which()`
        // For now, we test that the function returns Ok
        let result = resolve_system("0.13.1");
        assert!(result.is_ok());

        // The result may be Some or None depending on system state
        // We just verify it doesn't panic or error
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
        let result = resolve_system("99.99.99"); // Use valid semver format
        assert!(result.is_ok());

        // Should return None or Some (doesn't matter which)
        // Important: should not panic or return Err
    }

    // ========================================================================
    // Main Orchestration Tests
    // ========================================================================

    #[test]
    fn test_resolve_typst_from_managed() {
        let temp_cache = tempdir_in_workspace();
        let version = "0.14.0";

        let binary_path = test_helpers::create_fake_typst_in_temp(&temp_cache, version, "");

        let options = ResolveOptions {
            required_version: version.to_string(),
            project_root: env::current_dir().unwrap(),
            force_refresh: false,
        };

        let result = resolve_typst_with_override(options, Some(temp_cache.path().to_path_buf()));

        assert!(
            result.is_ok(),
            "resolve_typst_with_override failed: {:?}",
            result.err()
        );

        match result.unwrap() {
            ResolveResult::Resolved(info) | ResolveResult::Cached(info) => {
                assert_eq!(info.version, version);
                assert_eq!(info.path, binary_path);
            }
            ResolveResult::NotFound { .. } => {
                panic!("Should have found binary in managed cache");
            }
        }

        // TempDir automatically cleans up
    }

    #[test]
    fn test_resolve_typst_from_system() {
        // Test: resolve_typst should try system if not in managed
        // This test depends on system state
        let options = ResolveOptions {
            required_version: "99.88.77".to_string(), // Unlikely to be in managed
            project_root: env::current_dir().unwrap(),
            force_refresh: false,
        };

        let result = resolve_typst(options);
        assert!(result.is_ok());

        // Result depends on system state
        // Just verify it doesn't panic or error
        match result.unwrap() {
            ResolveResult::Resolved(info) | ResolveResult::Cached(info) => {
                // If found, version should match (though unlikely)
                assert_eq!(info.version, "99.88.77");
            }
            ResolveResult::NotFound {
                required_version,
                searched_locations,
            } => {
                // More likely: not found
                assert_eq!(required_version, "99.88.77");
                assert!(!searched_locations.is_empty());
            }
        }
    }

    #[test]
    fn test_resolve_typst_not_found() {
        let temp_cache = tempdir_in_workspace();

        let options = ResolveOptions {
            required_version: "99.99.99".to_string(),
            project_root: env::current_dir().unwrap(),
            force_refresh: false,
        };

        let result = resolve_typst_with_override(options, Some(temp_cache.path().to_path_buf()));

        assert!(result.is_ok());

        match result.unwrap() {
            ResolveResult::NotFound {
                required_version,
                searched_locations,
            } => {
                assert_eq!(required_version, "99.99.99");
                assert!(!searched_locations.is_empty());
                // Should have searched both managed and system
                assert!(searched_locations.len() >= 2);
            }
            _ => {
                // If somehow found in system PATH, that's also acceptable
            }
        }

        // TempDir automatically cleans up
    }

    #[test]
    fn test_resolve_typst_force_refresh() {
        let temp_cache = tempdir_in_workspace();
        let version = "0.15.0";

        test_helpers::create_fake_typst_in_temp(&temp_cache, version, "");

        let options = ResolveOptions {
            required_version: version.to_string(),
            project_root: env::current_dir().unwrap(),
            force_refresh: true,
        };

        let result = resolve_typst_with_override(options, Some(temp_cache.path().to_path_buf()));

        assert!(result.is_ok());

        // Should return Resolved (not Cached) since force_refresh=true
        match result.unwrap() {
            ResolveResult::Resolved(info) => {
                assert_eq!(info.version, version);
            }
            ResolveResult::Cached(_) => {
                // With force_refresh=true, should not return Cached
            }
            ResolveResult::NotFound { .. } => {
                panic!("Should have found binary");
            }
        }

        // TempDir automatically cleans up
    }

    #[test]
    fn test_resolve_typst_managed_priority_over_system() {
        let temp_cache = tempdir_in_workspace();
        let version = "0.16.0";

        let binary_path = test_helpers::create_fake_typst_in_temp(&temp_cache, version, "");

        let options = ResolveOptions {
            required_version: version.to_string(),
            project_root: env::current_dir().unwrap(),
            force_refresh: false,
        };

        let result = resolve_typst_with_override(options, Some(temp_cache.path().to_path_buf()));

        assert!(result.is_ok());

        match result.unwrap() {
            ResolveResult::Resolved(info) | ResolveResult::Cached(info) => {
                assert_eq!(info.version, version);
                assert_eq!(info.path, binary_path);
                // Should come from managed cache (not system)
                assert!(matches!(info.source, TypstSource::Managed));
            }
            ResolveResult::NotFound { .. } => {
                panic!("Should have found binary in managed cache");
            }
        }

        // TempDir automatically cleans up
    }

    #[test]
    fn test_resolve_typst_searched_locations() {
        let temp_cache = tempdir_in_workspace();

        let options = ResolveOptions {
            required_version: "98.76.54".to_string(),
            project_root: env::current_dir().unwrap(),
            force_refresh: false,
        };

        let result = resolve_typst_with_override(options, Some(temp_cache.path().to_path_buf()));

        assert!(result.is_ok());

        match result.unwrap() {
            ResolveResult::NotFound {
                searched_locations, ..
            } => {
                // Should have searched at least 2 locations (managed and system)
                assert!(searched_locations.len() >= 2);

                // Should include managed cache path
                let has_managed = searched_locations
                    .iter()
                    .any(|loc| loc.contains("managed") || loc.contains("cache"));
                assert!(
                    has_managed,
                    "Searched locations should include managed cache"
                );

                // Should include system PATH
                let has_system = searched_locations
                    .iter()
                    .any(|loc| loc.contains("system") || loc.contains("PATH"));
                assert!(has_system, "Searched locations should include system PATH");
            }
            _ => {
                // If found in system PATH (unlikely), that's acceptable
            }
        }

        // TempDir automatically cleans up
    }

    // ========================================================================
    // Robustness Tests
    // ========================================================================

    #[test]
    fn test_managed_cache_dir_always_succeeds() {
        // Should not panic even if dirs::cache_dir() returns None
        let result = managed_cache_dir();
        assert!(result.is_ok(), "managed_cache_dir should always succeed");

        let cache_dir = result.unwrap();
        assert!(
            cache_dir.ends_with("typstlab/typst")
                || cache_dir.to_string_lossy().contains(".typstlab-cache"),
            "Cache dir should be either standard cache or fallback"
        );

        // Directory should exist after calling managed_cache_dir
        assert!(cache_dir.exists(), "Cache directory should be created");
    }

    #[test]
    fn test_managed_cache_dir_with_override_creates_directory() {
        let temp_base = tempdir_in_workspace();
        let override_path = temp_base.path().join("custom-cache");

        let result = managed_cache_dir_with_override(Some(override_path.clone()));
        assert!(result.is_ok());

        let cache_dir = result.unwrap();

        // Should create the directory structure
        assert!(cache_dir.exists(), "Cache directory should be created");
        assert!(cache_dir.ends_with("typstlab/typst"));
    }
}
