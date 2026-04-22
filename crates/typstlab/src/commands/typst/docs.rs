//! Typst documentation management commands (thin endpoint layer)

use typstlab_core::context::Context;
use anyhow::{Result, anyhow};
use chrono::Utc;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use typstlab_core::config::NetworkPolicy;
use typstlab_core::state::{DocsState, TypstDocsInfo};
use typstlab_typst::docs;

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
pub fn sync(verbose: bool) -> Result<()> {
    let ctx = Context::builder().verbose(verbose).build()?;
    sync_with_context(&ctx)
}

pub fn sync_with_context(ctx: &Context) -> Result<()> {
    check_network_policy(ctx)?;

    let config = ctx.config.as_ref().ok_or_else(|| anyhow!("Config not found"))?;
    let version = &config.typst.version;

    if ctx.verbose {
        eprintln!("Syncing Typst {} documentation...", version);
    }

    let docs_dir = ctx.project.as_ref().ok_or_else(|| anyhow!("Project not found"))?.root.join(".typstlab/kb/typst/docs");

    cleanup_existing_docs(&docs_dir, ctx.verbose)?;

    // Delegate to library with env
    docs::sync_docs_with_env(&ctx.env, version, &docs_dir, ctx.verbose)?;

    update_state_synced(ctx)?;

    println!("{}", "Documentation synced successfully".green());

    Ok(())
}

/// Clear (remove) local Typst documentation
pub fn clear(verbose: bool) -> Result<()> {
    let ctx = Context::builder().verbose(verbose).build()?;
    clear_with_context(&ctx)
}

pub fn clear_with_context(ctx: &Context) -> Result<()> {
    let docs_dir = ctx.project.as_ref().ok_or_else(|| anyhow!("Project not found"))?.root.join(".typstlab/kb/typst/docs");

    if ctx.verbose {
        eprintln!("Clearing documentation at {}...", docs_dir.display());
    }

    cleanup_existing_docs(&docs_dir, ctx.verbose)?;

    update_state_cleared(ctx)?;

    println!("{}", "Documentation cleared successfully".green());

    Ok(())
}

/// Show Typst documentation status
pub fn status(json: bool, verbose: bool) -> Result<()> {
    let ctx = Context::builder().verbose(verbose).build()?;
    status_with_context(&ctx, json)
}

pub fn status_with_context(ctx: &Context, json: bool) -> Result<()> {
    let docs_dir = ctx.project.as_ref().ok_or_else(|| anyhow!("Project not found"))?.root.join(".typstlab/kb/typst/docs");
    let docs_present = docs_dir.exists();

    // Get info from state.json if available
    let docs_info = ctx.state.as_ref().and_then(|s| s.docs.as_ref()).and_then(|d| d.typst.as_ref());

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

// ============================================================================
// Helper functions (focused, single responsibility)
// ============================================================================

/// Checks network policy allows sync operations
fn check_network_policy(ctx: &Context) -> Result<()> {
    if ctx.config.as_ref().ok_or_else(|| anyhow!("Config not found"))?.network.policy == NetworkPolicy::Never {
        Err(anyhow!(
            "Cannot sync documentation: network policy is set to 'never'"
        ))
    } else {
        Ok(())
    }
}

/// Removes existing documentation directory if it exists
fn cleanup_existing_docs(dir: &Path, verbose: bool) -> Result<()> {
    if dir.exists() {
        if verbose {
            eprintln!("Removing existing documentation...");
        }
        fs::remove_dir_all(dir)?;
    }
    Ok(())
}

/// Updates state.json to reflect successfully synced documentation
fn update_state_synced(ctx: &Context) -> Result<()> {
    let mut state = ctx.state.clone().unwrap_or_default();
    state.docs = Some(DocsState {
        typst: Some(TypstDocsInfo {
            present: true,
            version: ctx.config.as_ref().unwrap().typst.version.clone(),
            synced_at: Utc::now(),
            source: "official".to_string(),
        }),
    });

    let state_path = ctx.project.as_ref().unwrap().root.join(".typstlab/state.json");
    state.save(&state_path).map_err(anyhow::Error::from)?;
    Ok(())
}

/// Updates state.json to reflect cleared documentation
fn update_state_cleared(ctx: &Context) -> Result<()> {
    let mut state = ctx.state.clone().unwrap_or_default();
    state.docs = Some(DocsState {
        typst: Some(TypstDocsInfo {
            present: false,
            version: ctx.config.as_ref().unwrap().typst.version.clone(),
            synced_at: Utc::now(),
            source: "official".to_string(),
        }),
    });

    let state_path = ctx.project.as_ref().unwrap().root.join(".typstlab/state.json");
    state.save(&state_path).map_err(anyhow::Error::from)?;
    Ok(())
}

/// Prints human-readable status to stdout
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

#[cfg(test)]
mod tests {
    use super::*;
    use typstlab_testkit::temp_dir_in_workspace;
    use typstlab_core::project::init_project;

    #[test]
    fn test_docs_status_basic() {
        let temp = temp_dir_in_workspace();
        let project_dir = temp.path().to_path_buf();
        init_project(&project_dir).unwrap();

        let ctx = Context::builder()
            .project_root(project_dir)
            .build()
            .unwrap();

        status_with_context(&ctx, false).unwrap();
    }
}
