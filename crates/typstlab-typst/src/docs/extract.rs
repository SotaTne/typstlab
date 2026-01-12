//! Typst documentation extraction from archive

use crate::docs::download::DocsError;
use flate2::read::GzDecoder;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use tar::{Archive, Entry};

/// Extracts docs/ directory from Typst archive
///
/// # Security
///
/// - Rejects absolute paths
/// - Rejects parent directory traversal (..)
/// - Only extracts docs/ directory
/// - Verifies paths stay within target_dir
///
/// # Arguments
///
/// * `bytes` - Archive bytes (tar.gz format)
/// * `target_dir` - Directory to extract docs to
/// * `verbose` - Enable verbose output
///
/// # Returns
///
/// Number of files extracted
pub fn extract_docs_directory(
    bytes: &[u8],
    target_dir: &Path,
    verbose: bool,
) -> Result<usize, DocsError> {
    let gz = GzDecoder::new(bytes);
    let mut archive = Archive::new(gz);

    fs::create_dir_all(target_dir)?;

    let entries = archive.entries()?;
    let mut extracted_count = 0;

    for entry in entries {
        if let Some(count) = extract_docs_entry(entry?, target_dir)? {
            extracted_count += count;
        }
    }

    if verbose {
        eprintln!("Extracted {} files", extracted_count);
    }

    if extracted_count == 0 {
        return Err(DocsError::NoDocsFound);
    }

    Ok(extracted_count)
}

/// Extract a single entry if it's within docs/ directory
///
/// Returns Some(1) if a file was extracted, Some(0) if a directory was created, None if skipped
fn extract_docs_entry(
    mut entry: Entry<GzDecoder<&[u8]>>,
    target_dir: &Path,
) -> Result<Option<usize>, DocsError> {
    let path = entry.path()?;
    let components: Vec<_> = path.components().collect();

    // Check if path is within docs/ directory
    // GitHub archive format: typst-{version}/docs/...
    if components.len() < 2 {
        return Ok(None);
    }

    // Validate second component is "docs"
    let Component::Normal(second) = components[1] else {
        return Ok(None);
    };

    if second != "docs" {
        return Ok(None);
    }

    // Security: Validate no path traversal in remaining components
    for component in &components[1..] {
        match component {
            Component::Normal(_) => continue,
            Component::ParentDir | Component::RootDir => {
                return Err(DocsError::PathTraversal);
            }
            _ => return Ok(None),
        }
    }

    // Extract file, removing the archive prefix
    let relative_path: PathBuf = components[1..].iter().collect();
    let target_path = target_dir.join(&relative_path);

    // Ensure target path is still within target_dir (defense in depth)
    let canonical_target = target_path
        .canonicalize()
        .unwrap_or_else(|_| target_path.clone());
    let canonical_base = target_dir
        .canonicalize()
        .unwrap_or_else(|_| target_dir.to_path_buf());

    if !canonical_target.starts_with(&canonical_base) {
        return Err(DocsError::PathTraversal);
    }

    // Extract directory or file
    if entry.header().entry_type().is_dir() {
        fs::create_dir_all(&target_path)?;
        Ok(Some(0))
    } else {
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut output = fs::File::create(&target_path)?;
        let mut content = Vec::new();
        entry.read_to_end(&mut content)?;
        std::io::Write::write_all(&mut output, &content)?;

        Ok(Some(1))
    }
}
