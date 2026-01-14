//! Typst version command - show required and resolved versions

use anyhow::Result;
use serde::Serialize;
use typstlab_core::{
    project::Project,
    state::{ResolvedSource, State},
};

#[derive(Debug, Serialize)]
struct VersionInfo {
    required_version: String,
    resolved_version: Option<String>,
    resolved_source: Option<String>,
    resolved_path: Option<String>,
}

/// Execute `typstlab typst version` command
pub fn execute_version(json: bool) -> Result<()> {
    // Find project root
    let project = Project::from_current_dir()?;
    let root = &project.root;

    // Get required version from config
    let config = project.config();
    let required_version = &config.typst.version;

    // Try to load state to get resolved info
    let state_path = root.join(".typstlab").join("state.json");
    let state = if state_path.exists() {
        State::load(&state_path).ok()
    } else {
        None
    };

    let version_info = VersionInfo {
        required_version: required_version.clone(),
        resolved_version: state
            .as_ref()
            .and_then(|s| s.typst.as_ref())
            .map(|t| t.resolved_version.clone()),
        resolved_source: state.as_ref().and_then(|s| s.typst.as_ref()).map(|t| {
            match t.resolved_source {
                ResolvedSource::Managed => "managed".to_string(),
                ResolvedSource::System => "system".to_string(),
            }
        }),
        resolved_path: state
            .as_ref()
            .and_then(|s| s.typst.as_ref())
            .map(|t| t.resolved_path.display().to_string()),
    };

    if json {
        // JSON output
        println!("{}", serde_json::to_string_pretty(&version_info)?);
    } else {
        // Human-readable output
        println!("Typst Version Information");
        println!("========================");
        println!();
        println!("Required: {}", version_info.required_version);

        if let Some(resolved_version) = &version_info.resolved_version {
            println!("Resolved: {}", resolved_version);

            if let Some(source) = &version_info.resolved_source {
                println!("Source:   {}", source);
            }

            if let Some(path) = &version_info.resolved_path {
                println!("Path:     {}", path);
            }

            // Check version match
            if resolved_version != &version_info.required_version {
                println!();
                println!("⚠ Warning: Resolved version does not match required version");
                println!("  Run `typstlab typst link --force` to refresh");
            }
        } else {
            println!();
            println!("Status: Not resolved");
            println!("Run `typstlab typst link` to resolve Typst");
        }
    }

    Ok(())
}
