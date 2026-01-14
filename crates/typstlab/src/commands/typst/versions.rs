//! Typst versions command - list all installed Typst versions

use anyhow::Result;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;
use typstlab_core::state::State;
use typstlab_typst::managed_cache_dir;

/// A single Typst version entry
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VersionEntry {
    /// Semantic version string (e.g., "0.14.0")
    pub version: String,
    /// Source of this version (managed or system)
    pub source: VersionSource,
    /// Absolute path to the binary
    pub path: PathBuf,
    /// Whether this is the currently active version
    pub is_current: bool,
}

/// Source of a Typst version
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VersionSource {
    /// Installed in managed cache
    Managed,
    /// Found in system PATH
    System,
}

/// List of all installed Typst versions
#[derive(Debug, Serialize)]
pub struct VersionsList {
    /// Array of version entries
    pub versions: Vec<VersionEntry>,
}

/// Execute `typstlab typst versions` command
pub fn execute_versions(json: bool) -> Result<()> {
    // 1. Collect all versions
    let versions = collect_all_versions()?;

    // 2. Output
    if json {
        format_json(&versions)?;
    } else {
        format_human_readable(&versions)?;
    }

    Ok(())
}

/// Collect all installed Typst versions
fn collect_all_versions() -> Result<Vec<VersionEntry>> {
    // 1. Scan managed cache
    let mut versions = scan_managed_versions()?;

    // 2. Detect system versions
    if let Some(system_entry) = detect_system_version()? {
        versions.push(system_entry);
    }

    // 3. Deduplicate (managed priority)
    versions = deduplicate_versions(versions);

    // 4. Load state.json to mark current version
    versions = mark_current_version(versions)?;

    // 5. Sort by semver (descending)
    versions = sort_versions(versions)?;

    Ok(versions)
}

/// Scan managed cache directory for installed versions
fn scan_managed_versions() -> Result<Vec<VersionEntry>> {
    let cache_dir = managed_cache_dir()?;

    if !cache_dir.exists() {
        return Ok(Vec::new());
    }

    let mut versions = Vec::new();

    for entry in std::fs::read_dir(&cache_dir)? {
        let entry = entry?;
        if let Some(version_entry) = validate_managed_version_dir(&entry.path()) {
            versions.push(version_entry);
        }
    }

    Ok(versions)
}

/// Validate a managed version directory and create VersionEntry
fn validate_managed_version_dir(dir_path: &std::path::Path) -> Option<VersionEntry> {
    if !dir_path.is_dir() {
        return None;
    }

    // Get version from directory name
    let dir_name = dir_path.file_name()?.to_str()?;

    // Validate semver format
    semver::Version::parse(dir_name).ok()?;

    // Check binary exists
    #[cfg(windows)]
    let binary_path = dir_path.join("typst.exe");
    #[cfg(not(windows))]
    let binary_path = dir_path.join("typst");

    if !binary_path.exists() {
        return None;
    }

    Some(VersionEntry {
        version: dir_name.to_string(),
        source: VersionSource::Managed,
        path: binary_path,
        is_current: false,
    })
}

/// Detect system Typst version in PATH
fn detect_system_version() -> Result<Option<VersionEntry>> {
    // Use which to find typst in PATH
    let binary_path = match which::which("typst") {
        Ok(path) => path,
        Err(_) => return Ok(None),
    };

    // Extract version
    let version = match extract_version(&binary_path)? {
        Some(v) => v,
        None => return Ok(None),
    };

    Ok(Some(VersionEntry {
        version,
        source: VersionSource::System,
        path: binary_path,
        is_current: false,
    }))
}

/// Extract version from Typst binary
fn extract_version(binary_path: &PathBuf) -> Result<Option<String>> {
    use std::process::Command;

    let output = Command::new(binary_path).arg("--version").output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return Ok(None),
    };

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse "typst X.Y.Z" format
    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("typst") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let version_str = parts[1].trim_start_matches('v');
                if semver::Version::parse(version_str).is_ok() {
                    return Ok(Some(version_str.to_string()));
                }
            }
        }
    }

    Ok(None)
}

/// Deduplicate versions (managed priority)
fn deduplicate_versions(versions: Vec<VersionEntry>) -> Vec<VersionEntry> {
    let mut seen_versions: HashMap<String, VersionEntry> = HashMap::new();

    for entry in versions {
        let version_key = entry.version.clone();

        if let Some(existing) = seen_versions.get(&version_key) {
            // If managed exists, keep it; otherwise keep system
            if existing.source == VersionSource::Managed {
                continue; // Keep existing managed
            } else if entry.source == VersionSource::Managed {
                // Replace system with managed
                seen_versions.insert(version_key, entry);
            }
        } else {
            seen_versions.insert(version_key, entry);
        }
    }

    seen_versions.into_values().collect()
}

/// Mark current version based on state.json
fn mark_current_version(mut versions: Vec<VersionEntry>) -> Result<Vec<VersionEntry>> {
    // Try to load state.json from current directory
    let state_path = std::env::current_dir()?
        .join(".typstlab")
        .join("state.json");

    if !state_path.exists() {
        // No state.json, all versions remain is_current=false
        return Ok(versions);
    }

    let state = match State::load(&state_path) {
        Ok(s) => s,
        Err(_) => return Ok(versions), // Ignore errors
    };

    let current_path = match &state.typst {
        Some(t) => &t.resolved_path,
        None => return Ok(versions),
    };

    // Mark matching entry
    for entry in &mut versions {
        if entry.path == *current_path {
            entry.is_current = true;
            break;
        }
    }

    Ok(versions)
}

/// Sort versions by semver (descending, newest first)
fn sort_versions(mut versions: Vec<VersionEntry>) -> Result<Vec<VersionEntry>> {
    versions.sort_by(|a, b| {
        let a_ver = semver::Version::parse(&a.version).ok();
        let b_ver = semver::Version::parse(&b.version).ok();

        match (a_ver, b_ver) {
            (Some(av), Some(bv)) => bv.cmp(&av), // Descending order
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => b.version.cmp(&a.version), // Descending string order
        }
    });

    Ok(versions)
}

/// Format human-readable output
fn format_human_readable(versions_list: &[VersionEntry]) -> Result<()> {
    println!("Typst versions:");

    if versions_list.is_empty() {
        println!("  (none installed)");
        return Ok(());
    }

    for entry in versions_list {
        let marker = if entry.is_current { "*" } else { " " };
        let local_suffix = if entry.source == VersionSource::System {
            " (local)"
        } else {
            ""
        };

        println!("  {} {}{}", marker, entry.version, local_suffix);
    }

    Ok(())
}

/// Format JSON output
fn format_json(versions_list: &[VersionEntry]) -> Result<()> {
    let output = VersionsList {
        versions: versions_list.to_vec(),
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
