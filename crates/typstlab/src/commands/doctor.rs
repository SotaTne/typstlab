//! Doctor command - environment health check

use crate::context::Context;
use anyhow::Result;

/// Run environment health check
///
/// # Arguments
///
/// * `json` - Output in JSON format if true
/// * `verbose` - Enable verbose output if true
///
/// # Returns
///
/// Always returns Ok(()) - doctor command always exits 0
pub fn run(_json: bool, verbose: bool) -> Result<()> {
    // Stub implementation for TDD Green phase (Commit 4)
    // Full implementation will be in Commit 6

    // Load context to verify we're in a project
    let _ctx = Context::new(verbose)?;

    // Output minimal valid JSON for now
    println!(r#"{{"schema_version":"1.0","checks":[]}}"#);

    Ok(())
}
