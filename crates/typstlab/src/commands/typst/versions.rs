use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use typstlab_core::project::Project;
use typstlab_core::state::State;
use typstlab_typst::resolve::managed_cache_dir;

#[derive(Serialize)]
struct VersionEntry {
    version: String,
    source: String,
    path: PathBuf,
    is_current: bool,
}

/// Execute `typstlab typst versions` command
pub fn execute_versions(json: bool) -> Result<()> {
    // 1. Find project and state to identify current version
    let project = Project::from_current_dir().ok();
    let state = project.and_then(|p| State::load(p.root.join(".typstlab/state.json")).ok());
    let current_path = state.and_then(|s| s.typst.map(|t| t.resolved_path));

    let mut entries = Vec::new();

    // 2. Add managed versions from cache
    let cache_dir = managed_cache_dir().context("Failed to get managed cache directory")?;
    if cache_dir.exists() {
        for entry in fs::read_dir(cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(version) = path.file_name().and_then(|n| n.to_str()) {
                    let binary_path = path.join("typst");
                    #[cfg(windows)]
                    let binary_path = path.join("typst.exe");

                    if binary_path.exists() {
                        let is_current = current_path.as_ref() == Some(&binary_path);
                        entries.push(VersionEntry {
                            version: version.to_string(),
                            source: "managed".to_string(),
                            path: binary_path,
                            is_current,
                        });
                    }
                }
            }
        }
    }

    // 3. Add system version from PATH
    if let Ok(system_path) = which::which("typst") {
        // Try to get version by running `typst --version`
        let output = std::process::Command::new(&system_path)
            .arg("--version")
            .output();

        let version = if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .split_whitespace()
                .nth(1)
                .unwrap_or("unknown")
                .to_string()
        } else {
            "unknown".to_string()
        };

        // Avoid duplicates if a managed version is somehow in PATH (unlikely but possible)
        if !entries.iter().any(|e| e.path == system_path) {
            let is_current = current_path.as_ref() == Some(&system_path);
            entries.push(VersionEntry {
                version,
                source: "system".to_string(),
                path: system_path,
                is_current,
            });
        }
    }

    // Sort descending by semver, fallback to string sort
    entries.sort_by(|a, b| {
        let v_a = semver::Version::parse(a.version.strip_prefix('v').unwrap_or(&a.version)).ok();
        let v_b = semver::Version::parse(b.version.strip_prefix('v').unwrap_or(&b.version)).ok();
        match (v_a, v_b) {
            (Some(va), Some(vb)) => vb.cmp(&va),
            _ => b.version.cmp(&a.version),
        }
    });

    if json {
        let output = serde_json::json!({
            "versions": entries
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if entries.is_empty() {
            println!("No Typst versions found.");
        } else {
            println!("{}", "Typst versions:".bold());
            for entry in &entries {
                let current_marker = if entry.is_current {
                    "*".green()
                } else {
                    " ".into()
                };

                let source_marker = if entry.source == "system" {
                    "(local)".dimmed()
                } else {
                    "(managed)".dimmed()
                };

                println!(
                    "{} {:<10} {:<10} {}",
                    current_marker,
                    entry.version.cyan(),
                    source_marker,
                    entry.path.display().to_string().dimmed()
                );
            }
        }
    }

    Ok(())
}
