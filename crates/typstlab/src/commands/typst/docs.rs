//! Typst documentation management commands

use crate::context::Context;
use anyhow::{Result, anyhow};
use chrono::Utc;
use colored::Colorize;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::Archive;
use typstlab_core::config::NetworkPolicy;
use typstlab_core::state::{DocsState, TypstDocsInfo};

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

    // Download and extract documentation
    let docs_dir = ctx.project.root.join(".typstlab/kb/typst/docs");
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
    // Construct GitHub archive URL
    let url = format!(
        "https://github.com/typst/typst/archive/refs/tags/v{}.tar.gz",
        version
    );

    if verbose {
        eprintln!("Downloading from {}...", url);
    }

    // Download tar.gz
    let response = reqwest::blocking::get(&url)?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to download documentation: HTTP {}",
            response.status()
        ));
    }

    // Read response body
    let bytes = response.bytes()?;

    if verbose {
        eprintln!("Downloaded {} bytes, extracting...", bytes.len());
    }

    // Decompress gzip
    let gz = GzDecoder::new(&bytes[..]);
    let mut archive = Archive::new(gz);

    // Create target directory
    fs::create_dir_all(target_dir)?;

    // Extract only docs/ directory
    let entries = archive.entries()?;
    let mut extracted_count = 0;

    for entry in entries {
        let mut entry = entry?;
        let path = entry.path()?;

        // Check if path is within docs/ directory
        // GitHub archive format: typst-{version}/docs/...
        let components: Vec<_> = path.components().collect();

        if components.len() >= 2
            && let Some(component_str) = components[1].as_os_str().to_str()
            && component_str == "docs"
        {
            // Extract file, removing the archive prefix
            let relative_path: PathBuf = components[1..].iter().collect();
            let target_path = target_dir.join(&relative_path);

            // Check if this is a directory or file
            if entry.header().entry_type().is_dir() {
                // Create directory
                fs::create_dir_all(&target_path)?;
            } else {
                // Create parent directories
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                // Extract file
                let mut output = fs::File::create(&target_path)?;
                let mut content = Vec::new();
                entry.read_to_end(&mut content)?;
                std::io::Write::write_all(&mut output, &content)?;

                extracted_count += 1;
            }
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
