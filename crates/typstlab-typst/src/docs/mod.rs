//! Typst documentation download and management

pub mod download;
pub mod extract;

use std::path::Path;

// Re-exports
pub use download::{DocsError, MAX_DOCS_SIZE, build_docs_archive_url, download_docs_archive};
pub use extract::extract_docs_directory;

/// High-level API: Download and extract Typst documentation
///
/// # Arguments
///
/// * `version` - Typst version (e.g., "0.12.0")
/// * `target_dir` - Directory to extract docs to
/// * `verbose` - Enable verbose output
///
/// # Returns
///
/// Number of files extracted
///
/// # Errors
///
/// Returns error if:
/// - URL construction fails
/// - Download fails
/// - Archive extraction fails
/// - No documentation files found
///
/// # Example
///
/// ```no_run
/// # use typstlab_typst::docs;
/// # use std::path::Path;
/// let count = docs::sync_docs("0.12.0", Path::new(".typstlab/kb/typst/docs"), false).unwrap();
/// println!("Extracted {} files", count);
/// ```
pub fn sync_docs(version: &str, target_dir: &Path, verbose: bool) -> Result<usize, DocsError> {
    // Download archive
    let bytes = download::download_docs_archive(version, verbose)?;

    // Extract docs/
    let count = extract::extract_docs_directory(&bytes, target_dir, verbose)?;

    Ok(count)
}
