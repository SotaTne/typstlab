//! Typst documentation management commands

use crate::context::Context;
use anyhow::{Result, anyhow};
use chrono::Utc;
use colored::Colorize;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tar::{Archive, Entry};
use typstlab_core::config::NetworkPolicy;
use typstlab_core::state::{DocsState, TypstDocsInfo};

/// Maximum download size (50 MB)
const MAX_DOWNLOAD_SIZE: u64 = 50 * 1024 * 1024;

/// Network timeout (30 seconds)
const NETWORK_TIMEOUT: Duration = Duration::from_secs(30);

/// Documentation status output schema
#[derive(Debug, Serialize, Deserialize)]
struct DocsStatus {
    present: bool,
    version: Option<String>,
    synced_at: Option<String>,
    source: Option<String>,
    path: Option<String>,
}

/// Sync (download) Typst documentation
///
/// # Arguments
///
/// * `verbose` - Enable verbose output if true
pub fn sync(verbose: bool) -> Result<()> {
    let ctx = Context::new(verbose)?;

    // Check network policy
    if ctx.config.network.policy == NetworkPolicy::Never {
        return Err(anyhow!(
            "Cannot sync documentation: network policy is set to 'never'"
        ));
    }

    let version = &ctx.config.typst.version;

    if verbose {
        eprintln!("Syncing Typst {} documentation...", version);
    }

    let docs_dir = ctx.project.root.join(".typstlab/kb/typst/docs");

    // Clean up existing docs before re-sync
    if docs_dir.exists() {
        if verbose {
            eprintln!("Removing existing documentation...");
        }
        fs::remove_dir_all(&docs_dir)?;
    }

    // Download and extract documentation
    download_and_extract_docs(version, &docs_dir, verbose)?;

    // Update state.json
    let mut state = ctx.state.clone();
    state.docs = Some(DocsState {
        typst: Some(TypstDocsInfo {
            present: true,
            version: version.clone(),
            synced_at: Utc::now(),
            source: "official".to_string(),
        }),
    });

    let state_path = ctx.project.root.join(".typstlab/state.json");
    state.save(&state_path)?;

    println!("{}", "Documentation synced successfully".green());

    Ok(())
}

/// Clear (remove) local Typst documentation
///
/// # Arguments
///
/// * `verbose` - Enable verbose output if true
pub fn clear(verbose: bool) -> Result<()> {
    let ctx = Context::new(verbose)?;

    let docs_dir = ctx.project.root.join(".typstlab/kb/typst/docs");

    if verbose {
        eprintln!("Clearing documentation at {}...", docs_dir.display());
    }

    // Remove docs directory if it exists
    if docs_dir.exists() {
        fs::remove_dir_all(&docs_dir)?;
    }

    // Update state.json
    let mut state = ctx.state.clone();
    state.docs = Some(DocsState {
        typst: Some(TypstDocsInfo {
            present: false,
            version: ctx.config.typst.version.clone(),
            synced_at: Utc::now(),
            source: "official".to_string(),
        }),
    });

    let state_path = ctx.project.root.join(".typstlab/state.json");
    state.save(&state_path)?;

    println!("{}", "Documentation cleared successfully".green());

    Ok(())
}

/// Show Typst documentation status
///
/// # Arguments
///
/// * `json` - Output in JSON format if true
/// * `verbose` - Enable verbose output if true
///
/// # Returns
///
/// Always returns Ok(()) - status command always exits 0
pub fn status(json: bool, verbose: bool) -> Result<()> {
    let ctx = Context::new(verbose)?;

    let docs_dir = ctx.project.root.join(".typstlab/kb/typst/docs");
    let docs_present = docs_dir.exists();

    // Get info from state.json if available
    let docs_info = ctx.state.docs.as_ref().and_then(|d| d.typst.as_ref());

    let status = DocsStatus {
        present: docs_present,
        version: docs_info.map(|i| i.version.clone()),
        synced_at: docs_info.map(|i| i.synced_at.to_rfc3339()),
        source: docs_info.map(|i| i.source.clone()),
        path: if docs_present {
            Some(docs_dir.display().to_string())
        } else {
            None
        },
    };

    if json {
        let json_str = serde_json::to_string_pretty(&status)?;
        println!("{}", json_str);
    } else {
        print_human_readable_status(&status);
    }

    Ok(())
}

/// Download and extract Typst documentation
fn download_and_extract_docs(version: &str, target_dir: &Path, verbose: bool) -> Result<()> {
    let bytes = download_archive(version, verbose)?;
    extract_docs_from_archive(&bytes, target_dir, verbose)?;
    Ok(())
}

/// Download archive from GitHub
fn download_archive(version: &str, verbose: bool) -> Result<Vec<u8>> {
    let url = format!(
        "https://github.com/typst/typst/archive/refs/tags/v{}.tar.gz",
        version
    );

    if verbose {
        eprintln!("Downloading from {}...", url);
    }

    // Create client with timeout
    let client = reqwest::blocking::Client::builder()
        .timeout(NETWORK_TIMEOUT)
        .build()?;

    let response = client.get(&url).send()?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to download documentation: HTTP {}",
            response.status()
        ));
    }

    // Check content length
    if let Some(content_length) = response.content_length()
        && content_length > MAX_DOWNLOAD_SIZE
    {
        return Err(anyhow!(
            "Download size ({} bytes) exceeds maximum ({} bytes)",
            content_length,
            MAX_DOWNLOAD_SIZE
        ));
    }

    let bytes = response.bytes()?.to_vec();

    if verbose {
        eprintln!("Downloaded {} bytes", bytes.len());
    }

    Ok(bytes)
}

/// Extract docs/ directory from archive
fn extract_docs_from_archive(bytes: &[u8], target_dir: &Path, verbose: bool) -> Result<()> {
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
        return Err(anyhow!("No documentation files found in archive"));
    }

    Ok(())
}

/// Extract a single entry if it's within docs/ directory
///
/// Returns Some(1) if a file was extracted, Some(0) if a directory was created, None if skipped
fn extract_docs_entry(
    mut entry: Entry<GzDecoder<&[u8]>>,
    target_dir: &Path,
) -> Result<Option<usize>> {
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
                return Err(anyhow!("Path traversal detected in archive entry"));
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
        return Err(anyhow!(
            "Path traversal detected: target outside base directory"
        ));
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

/// Print human-readable status
fn print_human_readable_status(status: &DocsStatus) {
    println!("{}", "Typst Documentation Status".bold());
    println!();

    if status.present {
        println!("  Status: {}", "present".green());

        if let Some(version) = &status.version {
            println!("  Version: {}", version);
        }

        if let Some(synced_at) = &status.synced_at {
            println!("  Synced at: {}", synced_at);
        }

        if let Some(source) = &status.source {
            println!("  Source: {}", source);
        }

        if let Some(path) = &status.path {
            println!("  Path: {}", path);
        }
    } else {
        println!("  Status: {}", "not present".red());
        println!();
        println!("  Run 'typstlab typst docs sync' to download documentation");
    }

    println!();
}
