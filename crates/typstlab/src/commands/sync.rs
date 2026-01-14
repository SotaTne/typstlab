//! `typstlab sync` - Synchronize project to build-ready state

use crate::context::Context;
use anyhow::Result;
use chrono::Utc;
use typstlab_core::project::generate_all_papers;
use typstlab_core::state::{State, SyncState};
use typstlab_core::status::engine::StatusEngine;
use typstlab_core::status::schema::SuggestedAction;

/// Run sync command
///
/// # Arguments
///
/// * `apply` - Apply doctor actions automatically (network, installs)
/// * `verbose` - Enable verbose output
///
/// # Returns
///
/// Result indicating success or failure
pub fn run(apply: bool, verbose: bool) -> Result<()> {
    let ctx = Context::new(verbose)?;

    if verbose {
        println!("→ Starting sync...");
    }

    // Run sync workflow
    if apply {
        sync_apply_mode(&ctx)?;
    } else {
        sync_default_mode(&ctx)?;
    }

    Ok(())
}

/// Execute default sync mode
///
/// Workflow:
/// 1. Resolve Typst (typst link)
/// 2. Generate layouts (generate --all)
/// 3. Update state.json (sync.last_sync)
fn sync_default_mode(ctx: &Context) -> Result<()> {
    // Step 1: Resolve Typst (typst link)
    println!("→ Resolving Typst...");
    run_typst_link(ctx)?;

    // Step 2: Generate all papers
    println!("\n→ Generating layouts...");
    run_generate_all(ctx)?;

    // Step 3: Update sync state
    if ctx.verbose {
        println!("→ Updating state...");
    }
    update_sync_state(ctx)?;

    println!("\n✓ Sync complete");

    Ok(())
}

/// Run typst link to resolve Typst binary
fn run_typst_link(_ctx: &Context) -> Result<()> {
    use crate::commands::typst::link::execute_link;

    let force = false; // Don't force re-link

    execute_link(force)?;

    // execute_link already prints verbose output
    Ok(())
}

/// Run generate --all to create _generated/ directories
fn run_generate_all(ctx: &Context) -> Result<()> {
    let generated = generate_all_papers(&ctx.project)?;

    if ctx.verbose {
        if generated.is_empty() {
            println!("! No papers found");
        } else {
            println!("✓ Generated {} paper(s)", generated.len());
        }
    }

    Ok(())
}

/// Update state.json with sync timestamp
fn update_sync_state(ctx: &Context) -> Result<()> {
    let state_path = ctx.project.root.join(".typstlab").join("state.json");

    // Load or create state
    let mut state = if state_path.exists() {
        State::load(&state_path)?
    } else {
        State::empty()
    };

    // Update sync section
    state.sync = Some(SyncState {
        last_sync: Some(Utc::now()),
    });

    // Save state
    state.save(&state_path)?;

    if ctx.verbose {
        println!("✓ State updated");
    }

    Ok(())
}

/// Execute apply sync mode
///
/// Workflow:
/// 1. Run default sync (typst link → generate → state update)
/// 2. Run status to get suggested actions
/// 3. Auto-execute allowed actions (v0.1 fixed)
fn sync_apply_mode(ctx: &Context) -> Result<()> {
    // Step 1: Run default sync workflow
    sync_default_mode(ctx)?;

    // Step 2: Run status to get suggested actions
    if ctx.verbose {
        println!("\n→ Checking for actions...");
    }

    let engine = StatusEngine::new();
    let report = engine.run(&ctx.project, None);

    // Step 3: Auto-execute allowed actions (v0.1 fixed)
    if !report.actions.is_empty() {
        apply_actions(ctx, &report.actions)?;
    } else if ctx.verbose {
        println!("✓ No actions needed");
    }

    Ok(())
}

/// Apply allowed actions automatically
///
/// v0.1: Only auto-execute typst install and docs sync
fn apply_actions(ctx: &Context, actions: &[SuggestedAction]) -> Result<()> {
    if ctx.verbose {
        println!("\n→ Applying fixes...");
    }

    for action in actions {
        match action {
            SuggestedAction::RunCommand {
                command,
                description,
            } => {
                apply_command_action(ctx, command, description)?;
            }
            _ => {
                // Skip InstallTool, CreateFile, EditFile
                if ctx.verbose {
                    println!("  • Skipping non-command action");
                }
            }
        }
    }

    Ok(())
}

/// Apply a single command action
fn apply_command_action(ctx: &Context, command: &str, description: &str) -> Result<()> {
    // Only allow specific commands (v0.1 fixed list)
    if command.starts_with("typstlab typst install ") {
        execute_typst_install(command)?;
    } else if command == "typstlab typst docs sync" {
        execute_docs_sync(ctx)?;
    } else if ctx.verbose {
        // Skip other commands
        println!("  • Skipping: {}", description);
    }
    Ok(())
}

/// Execute typst install command
fn execute_typst_install(command: &str) -> Result<()> {
    // Extract version from command
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.len() >= 4 {
        let version = parts[3];
        println!("  → Installing Typst {}...", version);

        use crate::commands::typst::install::execute_install;
        execute_install(version.to_string(), false)?;

        println!("  ✓ Typst {} installed", version);
    }
    Ok(())
}

/// Execute docs sync command
fn execute_docs_sync(ctx: &Context) -> Result<()> {
    println!("  → Syncing docs...");

    use crate::commands::typst::docs::sync;
    sync(ctx.verbose)?;

    println!("  ✓ Docs synced");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_state_structure() {
        let sync_state = SyncState {
            last_sync: Some(Utc::now()),
        };

        assert!(sync_state.last_sync.is_some());
    }
}
