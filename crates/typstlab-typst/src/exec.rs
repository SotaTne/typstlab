use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use typstlab_core::{Result, TypstlabError};
use crate::resolve::{resolve_typst, ResolveOptions, ResolveResult};

/// Options for executing Typst commands
#[derive(Debug, Clone)]
pub struct ExecOptions {
    pub project_root: PathBuf,
    pub args: Vec<String>,
    pub required_version: String,
}

/// Result of Typst command execution
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

// ============================================================================
// Helper Functions (to be implemented in Commit 9)
// ============================================================================

/// Execute command and capture output with timing
fn run_command(path: &Path, args: &[String]) -> Result<ExecResult> {
    let start = Instant::now();
    
    let output = Command::new(path)
        .args(args)
        .output()
        .map_err(|e| TypstlabError::TypstExecFailed(
            format!("Failed to execute command: {}", e)
        ))?;
    
    let duration_ms = start.elapsed().as_millis() as u64;
    
    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    Ok(ExecResult {
        exit_code,
        stdout,
        stderr,
        duration_ms,
    })
}

// ============================================================================
// Main Entry Point (to be implemented in Commit 9)
// ============================================================================

/// Execute a Typst command with the resolved binary
///
/// This function:
/// 1. Resolves the Typst binary using resolve_typst
/// 2. Executes the command with provided args
/// 3. Captures stdout, stderr, exit code, and timing
pub fn exec_typst(
    options: ExecOptions,
) -> Result<ExecResult> {
    // Step 1: Resolve the Typst binary
    let resolve_options = ResolveOptions {
        required_version: options.required_version.clone(),
        project_root: options.project_root.clone(),
        force_refresh: false,
    };
    
    let resolve_result = resolve_typst(resolve_options)?;
    
    // Step 2: Extract the binary path from resolve result
    let binary_path = match resolve_result {
        ResolveResult::Cached(info) => info.path,
        ResolveResult::Resolved(info) => info.path,
        ResolveResult::NotFound { required_version, searched_locations: _ } => {
            return Err(TypstlabError::TypstNotResolved {
                required_version,
            });
        }
    };
    
    // Step 3: Execute the command
    run_command(&binary_path, &options.args)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    // ========================================================================
    // Helper Function Tests
    // ========================================================================

    #[test]
    fn test_run_command_success() {
        // Create a fake binary that exits with success
        let temp_dir = env::temp_dir();
        let fake_binary = temp_dir.join("fake_typst_success");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(&fake_binary, "#!/bin/sh\necho 'success output'\nexit 0").unwrap();
            let mut perms = fs::metadata(&fake_binary).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_binary, perms).unwrap();
        }

        #[cfg(windows)]
        {
            let fake_binary = temp_dir.join("fake_typst_success.bat");
            fs::write(&fake_binary, "@echo success output\r\n@exit /b 0").unwrap();
        }

        let args = vec!["--version".to_string()];

        #[cfg(unix)]
        let result = run_command(&fake_binary, &args);
        #[cfg(windows)]
        let result = run_command(&temp_dir.join("fake_typst_success.bat"), &args);

        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert_eq!(exec_result.exit_code, 0);
        assert!(exec_result.stdout.contains("success output"));
        assert!(exec_result.duration_ms > 0);

        // Cleanup
        #[cfg(unix)]
        let _ = fs::remove_file(&fake_binary);
        #[cfg(windows)]
        let _ = fs::remove_file(temp_dir.join("fake_typst_success.bat"));
    }

    #[test]
    fn test_run_command_failure() {
        // Create a fake binary that exits with error
        let temp_dir = env::temp_dir();
        let fake_binary = temp_dir.join("fake_typst_failure");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(&fake_binary, "#!/bin/sh\necho 'error output' >&2\nexit 1").unwrap();
            let mut perms = fs::metadata(&fake_binary).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_binary, perms).unwrap();
        }

        #[cfg(windows)]
        {
            let fake_binary = temp_dir.join("fake_typst_failure.bat");
            fs::write(&fake_binary, "@echo error output 1>&2\r\n@exit /b 1").unwrap();
        }

        let args = vec!["compile".to_string()];

        #[cfg(unix)]
        let result = run_command(&fake_binary, &args);
        #[cfg(windows)]
        let result = run_command(&temp_dir.join("fake_typst_failure.bat"), &args);

        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert_eq!(exec_result.exit_code, 1);
        assert!(exec_result.stderr.contains("error output"));

        // Cleanup
        #[cfg(unix)]
        let _ = fs::remove_file(&fake_binary);
        #[cfg(windows)]
        let _ = fs::remove_file(temp_dir.join("fake_typst_failure.bat"));
    }

    #[test]
    fn test_run_command_binary_not_found() {
        let nonexistent = PathBuf::from("/nonexistent/path/to/typst");
        let args = vec!["--version".to_string()];

        let result = run_command(&nonexistent, &args);

        // Should return error when binary doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_run_command_captures_stdout_stderr() {
        // Create a fake binary that outputs to both stdout and stderr
        let temp_dir = env::temp_dir();
        let fake_binary = temp_dir.join("fake_typst_mixed");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(&fake_binary, "#!/bin/sh\necho 'stdout message'\necho 'stderr message' >&2\nexit 0").unwrap();
            let mut perms = fs::metadata(&fake_binary).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_binary, perms).unwrap();
        }

        #[cfg(windows)]
        {
            let fake_binary = temp_dir.join("fake_typst_mixed.bat");
            fs::write(&fake_binary, "@echo stdout message\r\n@echo stderr message 1>&2\r\n@exit /b 0").unwrap();
        }

        let args = vec![];

        #[cfg(unix)]
        let result = run_command(&fake_binary, &args);
        #[cfg(windows)]
        let result = run_command(&temp_dir.join("fake_typst_mixed.bat"), &args);

        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert!(exec_result.stdout.contains("stdout message"));
        assert!(exec_result.stderr.contains("stderr message"));

        // Cleanup
        #[cfg(unix)]
        let _ = fs::remove_file(&fake_binary);
        #[cfg(windows)]
        let _ = fs::remove_file(temp_dir.join("fake_typst_mixed.bat"));
    }

    #[test]
    fn test_run_command_timing() {
        // Test that duration_ms is measured
        let temp_dir = env::temp_dir();
        let fake_binary = temp_dir.join("fake_typst_timing");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(&fake_binary, "#!/bin/sh\nsleep 0.1\necho 'done'").unwrap();
            let mut perms = fs::metadata(&fake_binary).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_binary, perms).unwrap();
        }

        #[cfg(windows)]
        {
            let fake_binary = temp_dir.join("fake_typst_timing.bat");
            fs::write(&fake_binary, "@timeout /t 1 /nobreak >nul\r\n@echo done").unwrap();
        }

        let args = vec![];

        #[cfg(unix)]
        let result = run_command(&fake_binary, &args);
        #[cfg(windows)]
        let result = run_command(&temp_dir.join("fake_typst_timing.bat"), &args);

        assert!(result.is_ok());
        let exec_result = result.unwrap();
        // Should take at least some time
        assert!(exec_result.duration_ms > 0);

        // Cleanup
        #[cfg(unix)]
        let _ = fs::remove_file(&fake_binary);
        #[cfg(windows)]
        let _ = fs::remove_file(temp_dir.join("fake_typst_timing.bat"));
    }

    // ========================================================================
    // Main Function Tests
    // ========================================================================

    #[test]
    fn test_exec_typst_binary_not_resolved() {
        // Test: exec_typst should return error if binary cannot be resolved
        let options = ExecOptions {
            project_root: env::current_dir().unwrap(),
            args: vec!["--version".to_string()],
            required_version: "99.99.99".to_string(),
        };

        let result = exec_typst(options);

        // Should return error when binary is not found
        assert!(result.is_err());
    }

    #[test]
    fn test_exec_typst_with_resolved_binary() {
        // Setup: Create managed cache with valid binary
        use crate::resolve::managed_cache_dir;

        let cache_dir = managed_cache_dir().unwrap();
        let version = "0.17.0";
        let version_dir = cache_dir.join(version);

        fs::create_dir_all(&version_dir).unwrap();

        #[cfg(unix)]
        let binary_path = version_dir.join("typst");
        #[cfg(windows)]
        let binary_path = version_dir.join("typst.exe");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(&binary_path, format!("#!/bin/sh\necho 'typst {}'\nexit 0", version)).unwrap();
            let mut perms = fs::metadata(&binary_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&binary_path, perms).unwrap();
        }

        #[cfg(windows)]
        {
            fs::write(&binary_path, format!("@echo typst {}\r\n@exit /b 0", version)).unwrap();
        }

        // Test: exec_typst should execute the command
        let options = ExecOptions {
            project_root: env::current_dir().unwrap(),
            args: vec!["--version".to_string()],
            required_version: version.to_string(),
        };

        let result = exec_typst(options);

        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert_eq!(exec_result.exit_code, 0);
        assert!(exec_result.stdout.contains("typst"));

        // Cleanup
        let _ = fs::remove_dir_all(&version_dir);
    }

    #[test]
    fn test_exec_typst_preserves_exit_code() {
        // Setup: Create binary that exits with specific code
        use crate::resolve::managed_cache_dir;

        let cache_dir = managed_cache_dir().unwrap();
        let version = "0.18.0";
        let version_dir = cache_dir.join(version);

        fs::create_dir_all(&version_dir).unwrap();

        #[cfg(unix)]
        let binary_path = version_dir.join("typst");
        #[cfg(windows)]
        let binary_path = version_dir.join("typst.exe");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(&binary_path, format!("#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'typst {}'; exit 0; else exit 42; fi", version)).unwrap();
            let mut perms = fs::metadata(&binary_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&binary_path, perms).unwrap();
        }

        #[cfg(windows)]
        {
            fs::write(&binary_path, format!("@if \"%1\"==\"--version\" (echo typst {} && exit /b 0) else (exit /b 42)", version)).unwrap();
        }

        // Test: exec_typst with failing command
        let options = ExecOptions {
            project_root: env::current_dir().unwrap(),
            args: vec!["compile".to_string()],
            required_version: version.to_string(),
        };

        let result = exec_typst(options);

        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert_eq!(exec_result.exit_code, 42);

        // Cleanup
        let _ = fs::remove_dir_all(&version_dir);
    }
}
