//! Typst link command - resolve and link system or managed Typst

use anyhow::Result;
use std::path::Path;
use typstlab_core::{
    TypstlabError,
    project::Project,
    state::{ResolvedSource, State, TypstState},
};
use typstlab_typst::{ResolveOptions, ResolveResult, resolve_typst};

/// Execute `typstlab typst link` command
pub fn execute_link(force: bool) -> Result<()> {
    // Find project root
    let project = Project::from_current_dir()?;
    let root = &project.root;

    // Load config to get required version
    let config = project.config();
    let required_version = &config.typst.version;

    // Resolve Typst
    let resolve_opts = ResolveOptions {
        required_version: required_version.clone(),
        project_root: root.to_path_buf(),
        force_refresh: force,
    };

    let resolve_result = resolve_typst(resolve_opts)?;

    match resolve_result {
        ResolveResult::Cached(info) => {
            println!(
                "✓ Typst {} already resolved from {}",
                info.version, info.source
            );
            println!("  Path: {}", info.path.display());

            // Still need to create/update shim
            create_bin_shim(root, &info.path)?;
            update_state(root, &info.path, &info.version, info.source.to_string())?;
        }
        ResolveResult::Resolved(info) => {
            println!("✓ Typst {} resolved from {}", info.version, info.source);
            println!("  Path: {}", info.path.display());

            // Create shim
            create_bin_shim(root, &info.path)?;

            // Update state
            update_state(root, &info.path, &info.version, info.source.to_string())?;
        }
        ResolveResult::NotFound {
            required_version,
            searched_locations,
        } => {
            eprintln!("✗ Typst {} not found", required_version);
            eprintln!("\nSearched locations:");
            for location in searched_locations {
                eprintln!("  - {}", location);
            }
            eprintln!("\nTo install, run:");
            eprintln!("  typstlab typst install {}", required_version);
            return Err(TypstlabError::TypstNotResolved { required_version }.into());
        }
    }

    Ok(())
}

/// Create bin/typst shim
pub(super) fn create_bin_shim(project_root: &Path, resolved_path: &Path) -> Result<()> {
    let bin_dir = project_root.join("bin");
    std::fs::create_dir_all(&bin_dir)?;

    #[cfg(unix)]
    let shim_path = bin_dir.join("typst");
    #[cfg(windows)]
    let shim_path = bin_dir.join("typst.cmd");

    // Generate shim content
    let shim_content = generate_shim_content(project_root, resolved_path)?;

    std::fs::write(&shim_path, shim_content)?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&shim_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&shim_path, perms)?;
    }

    println!("✓ Created {}", shim_path.display());

    Ok(())
}

/// Generate shim script content
fn generate_shim_content(project_root: &Path, resolved_path: &Path) -> Result<String> {
    #[cfg(unix)]
    {
        Ok(format!(
            r#"#!/bin/sh
# typstlab-generated shim for Typst
# Project root: {}
# Resolved path: {}

exec typstlab typst exec -- "$@"
"#,
            project_root.display(),
            resolved_path.display()
        ))
    }

    #[cfg(windows)]
    {
        Ok(format!(
            r#"@echo off
REM typstlab-generated shim for Typst
REM Project root: {}
REM Resolved path: {}

typstlab typst exec -- %*
"#,
            project_root.display(),
            resolved_path.display()
        ))
    }
}

/// Update state.json with resolved Typst info
pub(super) fn update_state(
    project_root: &Path,
    resolved_path: &Path,
    version: &str,
    source: String,
) -> Result<()> {
    let typstlab_dir = project_root.join(".typstlab");
    std::fs::create_dir_all(&typstlab_dir)?;

    let state_path = typstlab_dir.join("state.json");

    // Load or create state
    let mut state = if state_path.exists() {
        State::load(&state_path)?
    } else {
        State::empty()
    };

    // Update typst section
    let resolved_source = match source.as_str() {
        "system" => ResolvedSource::System,
        "managed" => ResolvedSource::Managed,
        _ => ResolvedSource::System, // Default fallback
    };

    state.typst = Some(TypstState {
        resolved_path: resolved_path.to_path_buf(),
        resolved_version: version.to_string(),
        resolved_source,
        checked_at: chrono::Utc::now(),
    });

    // Save state
    state.save(&state_path)?;

    println!("✓ Updated state.json");

    Ok(())
}
