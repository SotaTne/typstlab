//! `typstlab sync` - Synchronize project to build-ready state

use crate::context::Context;
use anyhow::Result;
use chrono::Utc;
use typstlab_core::project::generate_all_papers;
use typstlab_core::state::{State, SyncState};

/// Run sync command
///
/// # Arguments
///
/// * `verbose` - Enable verbose output
///
/// # Returns
///
/// Result indicating success or failure
pub fn run(verbose: bool) -> Result<()> {
    let ctx = Context::new(verbose)?;

    if verbose {
        println!("→ Starting sync...");
    }

    // Run sync workflow
    sync_default_mode(&ctx)?;

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
