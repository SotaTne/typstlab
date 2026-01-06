use std::path::PathBuf;
use typstlab_core::Result;
use crate::info::TypstInfo;

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
    // TODO: Implement in Commit 6-7
    unimplemented!("resolve_typst will be implemented in commits 2-7")
}
