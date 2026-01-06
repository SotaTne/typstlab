use std::path::PathBuf;
use typstlab_core::Result;

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

/// Execute a Typst command with the resolved binary
pub fn exec_typst(
    _options: ExecOptions,
) -> Result<ExecResult> {
    // TODO: Implement in Commit 8-9
    unimplemented!("exec_typst will be implemented in commits 8-9")
}
