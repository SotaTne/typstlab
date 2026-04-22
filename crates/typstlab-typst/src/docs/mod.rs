//! Typst documentation download and management

pub mod download;
pub mod download_json;
pub mod generate;
pub mod html_to_md;
pub mod html_to_mdast;
pub mod links;
pub mod render;
pub mod render_bodies;
pub mod render_func;
pub mod schema;

use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::Duration;
use walkdir::WalkDir;

// Re-exports
pub use download::{DocsError, MAX_DOCS_SIZE};
pub use links::rewrite_docs_link;

/// High-level API: Download and generate Typst documentation
///
/// Downloads docs.json from typst-community/dev-builds and generates
/// hierarchical Markdown files for LLM consumption.
///
/// # Arguments
///
/// * `version` - Typst version (e.g., "0.12.0")
/// * `target_dir` - Directory to write Markdown files to
/// * `verbose` - Enable verbose output
///
/// # Returns
///
/// Number of files generated
///
/// # Errors
///
/// Returns error if:
/// - URL construction fails
/// - Download fails
/// - JSON parsing fails
/// - Schema validation fails
/// - Markdown generation fails
///
/// # Example
///
/// ```no_run
/// # use typstlab_typst::docs;
/// # use std::path::Path;
/// let count = docs::sync_docs("0.12.0", Path::new(".typstlab/kb/typst/docs"), false).unwrap();
/// println!("Generated {} files", count);
/// ```
/// High-level API: Download and generate Typst documentation
pub fn sync_docs(version: &str, target_dir: &Path, verbose: bool) -> Result<usize, DocsError> {
    let env = typstlab_core::context::Environment::from_env();
    sync_docs_with_env(&env, version, target_dir, verbose)
}

/// Download and generate Typst documentation using specific environment
pub fn sync_docs_with_env(
    env: &typstlab_core::context::Environment,
    version: &str,
    target_dir: &Path,
    verbose: bool,
) -> Result<usize, DocsError> {
    // 1. Get central cache directory
    let cache_root = env.docs_cache_dir();
    let cache_dir = cache_root.join(version);
    let manifest_path = cache_dir.join("manifest.lock");

    // 2. Prepare lock for central cache (atomicity across processes)
    let lock_path = cache_dir.join("sync.lock");
    fs::create_dir_all(&cache_dir)?;

    let _lock_guard = typstlab_core::lock::acquire_lock(
        &lock_path,
        Duration::from_secs(300), // 5 minutes for download + generation
        &format!("Syncing central docs cache for Typst {}", version),
    )
    .map_err(|e| DocsError::LockError(e.to_string()))?;

    // 3. Sync to central cache if needed
    if !manifest_path.exists() || !verify_manifest(&cache_dir, &manifest_path)? {
        if verbose {
            eprintln!("Central docs cache missing or invalid for version {}. Downloading...", version);
        }
        
        // Clean up partially generated files
        if cache_dir.exists() {
            let _ = fs::remove_dir_all(&cache_dir);
            fs::create_dir_all(&cache_dir)?;
        }

        // Download docs.json
        let json_bytes = download_json::download_docs_json(version, verbose)?;
        
        // Parse JSON
        let entries: Vec<schema::DocsEntry> = serde_json::from_slice(&json_bytes)?;
        
        // Generate Markdown files into cache
        generate::generate_markdown_files(&entries, &cache_dir, verbose)?;
        
        // Generate manifest.lock
        let manifest = generate_manifest(&cache_dir, &manifest_path)?;
        fs::write(&manifest_path, manifest)?;
    }

    // 4. Copy from central cache to target_dir (with verification)
    if verbose {
        eprintln!("Copying documentation to project: {}", target_dir.display());
    }
    
    // Acquire project-level lock for target_dir
    let project_lock_dir = target_dir.parent().and_then(|p| p.parent()).unwrap_or(target_dir);
    let project_lock = project_lock_dir.join("docs.lock");
    let _proj_lock_guard = typstlab_core::lock::acquire_lock(
        &project_lock,
        Duration::from_secs(60),
        "Installing docs to project",
    ).map_err(|e| DocsError::LockError(e.to_string()))?;

    // Copy files
    fs::create_dir_all(target_dir)?;
    let count = copy_with_verification(&cache_dir, target_dir, &manifest_path)?;

    Ok(count)
}

/// Generates a manifest of files and their SHA-256 hashes
fn generate_manifest(dir: &Path, manifest_path: &Path) -> Result<String, DocsError> {
    let mut manifest = String::new();
    for entry in WalkDir::new(dir) {
        let entry = entry.map_err(|e| DocsError::IoError(e.into()))?;
        if entry.file_type().is_file() {
            let path = entry.path();
            if path == manifest_path || path.extension().and_then(|s| s.to_str()) == Some("lock") {
                continue;
            }
            
            let rel_path = path.strip_prefix(dir).unwrap();
            let hash = compute_sha256(path)?;
            manifest.push_str(&format!("{} {}\n", hash, rel_path.display()));
        }
    }
    Ok(manifest)
}

/// Verifies that all files in the directory match the manifest
fn verify_manifest(dir: &Path, manifest_path: &Path) -> Result<bool, DocsError> {
    let content = fs::read_to_string(manifest_path)?;
    for line in content.lines() {
        if line.trim().is_empty() { continue; }
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() != 2 { continue; }
        
        let expected_hash = parts[0];
        let rel_path = parts[1];
        let full_path = dir.join(rel_path);
        
        if !full_path.exists() { return Ok(false); }
        let actual_hash = compute_sha256(&full_path)?;
        if actual_hash != expected_hash { return Ok(false); }
    }
    Ok(true)
}

/// Copies files from cache to project while verifying hashes
fn copy_with_verification(src: &Path, dst: &Path, manifest_path: &Path) -> Result<usize, DocsError> {
    let content = fs::read_to_string(manifest_path)?;
    let mut count = 0;
    for line in content.lines() {
        if line.trim().is_empty() { continue; }
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() != 2 { continue; }
        
        let expected_hash = parts[0];
        let rel_path = parts[1];
        let src_path = src.join(rel_path);
        let dst_path = dst.join(rel_path);
        
        // Verify source hash
        let actual_hash = compute_sha256(&src_path)?;
        if actual_hash != expected_hash {
            return Err(DocsError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Cache corruption detected: {}", rel_path)
            )));
        }
        
        // Ensure parent exists
        if let Some(parent) = dst_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Copy file
        fs::copy(&src_path, &dst_path)?;
        count += 1;
    }
    Ok(count)
}

/// Computes SHA-256 hash of a file
fn compute_sha256(path: &Path) -> Result<String, DocsError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Recursively count files (not directories) in a directory tree
///
/// This matches the behavior of generate_markdown_files() which only counts files.
/// Test helpers for docs module
pub mod test_helpers {
    use std::path::PathBuf;

    /// Load docs.json from fixtures
    ///
    /// Reads the docs.json file from project fixtures.
    /// This file contains the actual Typst documentation in JSON format.
    ///
    /// # Returns
    ///
    /// Binary content of docs.json
    ///
    /// # Panics
    ///
    /// Panics if the fixture file cannot be read
    pub fn load_docs_json_from_fixtures() -> Vec<u8> {
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
            .join("docs.json");

        std::fs::read(&fixtures_path).expect("Failed to read docs.json from fixtures")
    }

    /// Create a mock GitHub server with docs.json response
    ///
    /// # Arguments
    ///
    /// * `server` - mockito Server instance
    /// * `version` - Typst version (e.g., "0.12.0")
    /// * `json_bytes` - Binary content of docs.json
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
    /// let json_bytes = test_helpers::load_docs_json_from_fixtures();
    /// let mock = test_helpers::mock_github_docs_json(&mut server, "0.12.0", &json_bytes)
    ///     .expect(1)
    ///     .create();
    /// ```
    pub fn mock_github_docs_json(
        server: &mut mockito::Server,
        version: &str,
        json_bytes: &[u8],
    ) -> mockito::Mock {
        server
            .mock(
                "GET",
                format!(
                    "/typst-community/dev-builds/releases/download/docs-v{}/docs.json",
                    version
                )
                .as_str(),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json_bytes)
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
