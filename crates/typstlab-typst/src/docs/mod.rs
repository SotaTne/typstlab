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
    let lock_path = kb_dir.join("docs.lock");

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

/// Test helpers for docs module
pub mod test_helpers {
    use std::path::PathBuf;

    /// Load docs archive from fixtures
    ///
    /// Reads the pre-downloaded typst-0.12.0.tar.gz archive from project fixtures.
    /// This archive contains the actual Typst documentation.
    ///
    /// # Returns
    ///
    /// Binary content of the docs archive
    ///
    /// # Panics
    ///
    /// Panics if the fixture file cannot be read
    pub fn load_docs_archive_from_fixtures() -> Vec<u8> {
        let manifest_dir =
            std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set");
        let fixtures_path = PathBuf::from(manifest_dir)
            .parent() // crates/typstlab-typst -> crates
            .expect("Failed to get crates directory")
            .parent() // crates -> project root
            .expect("Failed to get project root")
            .join("fixtures")
            .join("typst")
            .join("v0.12.0")
            .join("typst-0.12.0.tar.gz");

        std::fs::read(&fixtures_path).expect("Failed to read typst-0.12.0.tar.gz from fixtures")
    }

    /// Create a mock GitHub server with release archive response
    ///
    /// # Arguments
    ///
    /// * `server` - mockito Server instance
    /// * `version` - Typst version (e.g., "0.12.0")
    /// * `asset_bytes` - Binary content of asset
    ///
    /// # Returns
    ///
    /// mockito Mock instance (call `.create()` to activate)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mockito::Server;
    /// use typstlab_typst::docs::test_helpers;
    ///
    /// let mut server = Server::new();
    /// let asset_bytes = test_helpers::load_docs_archive_from_fixtures();
    /// let mock = test_helpers::mock_github_docs_release(&mut server, "0.12.0", &asset_bytes)
    ///     .expect(1)
    ///     .create();
    /// ```
    pub fn mock_github_docs_release(
        server: &mut mockito::Server,
        version: &str,
        asset_bytes: &[u8],
    ) -> mockito::Mock {
        server
            .mock(
                "GET",
                format!("/typst/typst/archive/refs/tags/v{}.tar.gz", version).as_str(),
            )
            .with_status(200)
            .with_header("content-type", "application/gzip")
            .with_body(asset_bytes)
    }

    /// Set GitHub base URL to mock server for testing
    ///
    /// # Safety
    ///
    /// This function modifies environment variables, which is not thread-safe.
    /// Tests using this function should be run with `--test-threads=1`.
    ///
    /// # Arguments
    ///
    /// * `url` - Mock server URL (from `mockito::Server::url()`)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mockito::Server;
    /// use typstlab_typst::docs::test_helpers;
    ///
    /// let server = Server::new();
    /// test_helpers::set_mock_github_url(&server.url());
    /// // ... run tests ...
    /// test_helpers::clear_mock_github_url();
    /// ```
    pub fn set_mock_github_url(url: &str) {
        unsafe {
            std::env::set_var("GITHUB_BASE_URL", url);
        }
    }

    /// Clear mock GitHub base URL
    ///
    /// # Safety
    ///
    /// This function modifies environment variables, which is not thread-safe.
    /// Tests using this function should be run with `--test-threads=1`.
    pub fn clear_mock_github_url() {
        unsafe {
            std::env::remove_var("GITHUB_BASE_URL");
        }
    }
}
