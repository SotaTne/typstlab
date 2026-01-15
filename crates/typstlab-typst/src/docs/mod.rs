//! Typst documentation download and management

pub mod download;
pub mod extract;

use std::fs;
use std::path::Path;
use std::time::Duration;

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
    // Prepare lock path (.typstlab/kb/.lock)
    // target_dir is typically .typstlab/kb/typst/docs, so we need to go up 2 levels
    // to get to .typstlab/kb
    let kb_dir = target_dir
        .parent() // .typstlab/kb/typst
        .and_then(|p| p.parent()) // .typstlab/kb
        .ok_or_else(|| {
            DocsError::LockError("Target dir must be under kb/typst/docs".to_string())
        })?;
    let lock_path = kb_dir.join(".lock");

    // Acquire project-level lock (one sync at a time per project)
    // This prevents race conditions when multiple processes sync docs simultaneously
    let _lock_guard = typstlab_core::lock::acquire_lock(
        &lock_path,
        Duration::from_secs(120), // 2 minutes timeout for docs download
        &format!("Syncing documentation for Typst {}", version),
    )
    .map_err(|e| DocsError::LockError(e.to_string()))?;

    // Early exit if docs already exist (idempotency)
    if target_dir.exists() {
        // Count files recursively (matching extract behavior which only counts files, not dirs)
        let file_count = count_files_recursively(target_dir)?;
        if file_count > 0 {
            // Docs already synced, return count
            if verbose {
                eprintln!(
                    "Documentation for Typst {} already synced ({} files)",
                    version, file_count
                );
            }
            return Ok(file_count);
        }
    }

    // Download archive
    let bytes = download::download_docs_archive(version, verbose)?;

    // Extract docs/
    let count = extract::extract_docs_directory(&bytes, target_dir, verbose)?;

    // Lock automatically released when _lock_guard is dropped
    Ok(count)
}

/// Recursively count files (not directories) in a directory tree
///
/// This matches the behavior of extract_docs_directory() which only counts files.
fn count_files_recursively(dir: &Path) -> Result<usize, DocsError> {
    let mut count = 0;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            count += count_files_recursively(&path)?;
        } else {
            count += 1;
        }
    }
    Ok(count)
}
