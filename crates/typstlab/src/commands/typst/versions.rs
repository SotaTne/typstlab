use anyhow::Result;
use colored::Colorize;
use serde_json::json;
use std::fs;
use typstlab_typst::resolve::managed_cache_dir;

/// Execute `typstlab typst versions` command
pub fn execute_versions(json: bool) -> Result<()> {
    let cache_dir = managed_cache_dir()?;
    let mut versions = Vec::new();

    if cache_dir.exists() {
        for entry in fs::read_dir(cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Simple check if it looks like a version (start with digit or v)
                    // or just include all dirs for now.
                    // Typical versions: 0.12.0, 0.13.0
                    // Let's assume all dirs in cache root are versions.
                    versions.push(name.to_string());
                }
            }
        }
    }

    versions.sort();

    if json {
        let output = json!({
            "versions": versions
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if versions.is_empty() {
            println!("No managed Typst versions found.");
        } else {
            println!("Installed Typst versions:");
            for version in versions {
                println!("  {} {} (managed)", "•".cyan(), version);
            }
        }
    }

    Ok(())
}
