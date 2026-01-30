//! Build command - compile papers to PDF using Typst

use crate::context::Context;
use anyhow::{Result, bail};
use chrono::Utc;
use colored::Colorize;
use std::fs;
use std::time::Instant;
use typstlab_core::project::generate_paper;
use typstlab_core::state::{BuildState, LastBuild};
use typstlab_typst::exec::{ExecOptions, exec_typst};

/// Build paper to PDF
///
/// # Arguments
///
/// * `paper_id` - Paper ID to build (required)
/// * `full` - Force regenerate _generated/ before build
/// * `verbose` - Enable verbose output if true
/// Build all papers in the project
pub fn run_all(full: bool, verbose: bool) -> Result<()> {
    let ctx = Context::new(verbose)?;
    let papers = ctx.project.papers();

    if papers.is_empty() {
        println!("{} No papers found in project", "→".cyan());
        return Ok(());
    }

    use rayon::prelude::*;

    // Convert to Vec for parallel iteration
    let paper_ids: Vec<String> = papers.iter().map(|p| p.id().to_string()).collect();

    if verbose {
        println!(
            "{} Building {} papers in parallel",
            "→".cyan(),
            paper_ids.len()
        );
    }

    // Build independent Contexts or share read-only?
    // Context isn't easily cloneable if it holds state that mutates.
    // Ideally we pass reference to ctx to helpers.
    // Rayon requires Send/Sync. Context structure check:
    // Project is read-only mostly. State updates handle locking.
    // So passing &Context should be fine if Context is Sync.
    // But `exec_typst` might output to stdout, interleaved output is bad.
    // We should buffer output or use a progress bar separate library.
    // For now, let's keep it simple: just run parallel and let stdout interleave (imperfect but functional).

    // Actually, Context definition isn't shown but likely holds Config/Project which are fine.
    // But we need to construct it once.

    paper_ids.par_iter().for_each(|id| {
        // We catch errors to avoid stopping other builds
        if let Err(e) = build_paper(&ctx, id, full, verbose) {
            eprintln!("{} Failed to build paper '{}': {}", "✗".red().bold(), id, e);
        }
    });

    Ok(())
}

/// Build a specific paper
pub fn run(paper_id: String, full: bool, verbose: bool) -> Result<()> {
    let ctx = Context::new(verbose)?;
    build_paper(&ctx, &paper_id, full, verbose)
}

/// Core build logic (extracted for reuse and parallel execution)
fn build_paper(ctx: &Context, paper_id: &str, full: bool, verbose: bool) -> Result<()> {
    // Step 1: Find paper
    if verbose {
        println!("{} Finding paper '{}'", "→".cyan(), paper_id);
    }

    let paper = ctx
        .project
        .find_paper(paper_id)
        .ok_or_else(|| anyhow::anyhow!("Paper '{}' not found", paper_id))?;

    // Step 2: Check main file exists
    // if verbose { println!("{} Checking main file", "→".cyan()); }

    let main_file_path = paper.absolute_main_file_path();
    if !paper.has_main_file() {
        bail!(
            "Main file '{}' not found in paper '{}'",
            main_file_path.display(),
            paper_id
        );
    }

    // Step 3: Check root directory exists (if specified)
    if let Some(root_dir) = paper.typst_root_dir() {
        if !root_dir.exists() {
            bail!(
                "Root directory '{}' not found for paper '{}'",
                root_dir.display(),
                paper_id
            );
        }
    }

    // Step 4: Check/generate _generated/ directory
    let generated_dir = paper.generated_dir();
    if full || !generated_dir.exists() {
        if verbose {
            if full {
                println!(
                    "{} [{}] Forcing regeneration of _generated/",
                    "→".cyan(),
                    paper_id
                );
            } else {
                println!(
                    "{} [{}] Generating _generated/ (not found)",
                    "→".cyan(),
                    paper_id
                );
            }
        }

        // generate_paper reads project files, thread-safe for reading usually.
        generate_paper(&ctx.project, paper_id)?;
    }

    // Step 5: Create dist/<paper_id>/ directory
    let dist_dir = ctx.project.root.join("dist").join(paper_id);
    fs::create_dir_all(&dist_dir)?;

    // Step 6: Build Typst compile command
    let output_filename = format!("{}.pdf", paper.config().output.name);
    let output_path = dist_dir.join(&output_filename);

    let mut args = vec!["compile".to_string()];

    // Add --root option if specified
    if let Some(root_dir) = paper.typst_root_dir() {
        args.push("--root".to_string());
        args.push(root_dir.display().to_string());
    }

    args.push(main_file_path.display().to_string());
    args.push(output_path.display().to_string());

    // Step 7: Execute Typst compile
    if verbose {
        // println!("{} Compiling paper to PDF", "→".cyan());
        println!("{} [{}] Compiling...", "→".cyan(), paper_id);
    }

    let start_time = Instant::now();
    let start_utc = Utc::now();

    let exec_result = exec_typst(ExecOptions {
        project_root: ctx.project.root.clone(),
        args,
        required_version: ctx.config.typst.version.clone(),
    })?;

    let finish_utc = Utc::now();
    let duration = start_time.elapsed();
    let duration_ms = duration.as_millis() as u64;

    // Step 8: Check execution result
    let success = exec_result.exit_code == 0;

    if !success {
        eprintln!("{} [{}] Build failed", "✗".red().bold(), paper_id);
        if !exec_result.stderr.is_empty() {
            eprintln!("[{}] Stderr:\n{}", paper_id, exec_result.stderr);
        }

        // Update state with failure
        let build_state = BuildState {
            last: Some(LastBuild {
                paper: paper_id.to_string(),
                success: false,
                started_at: start_utc,
                finished_at: finish_utc,
                duration_ms,
                output: output_path.clone(),
                error: Some(exec_result.stderr.clone()),
            }),
        };

        let mut state = ctx.state.clone();
        state.build = Some(build_state);
        let state_path = ctx.project.root.join(".typstlab/state.json");
        state.save(&state_path)?;

        bail!(
            "[{}] Typst compilation failed with exit code {}",
            paper_id,
            exec_result.exit_code
        );
    }

    // Step 9: Update state.json with success
    let build_state = BuildState {
        last: Some(LastBuild {
            paper: paper_id.to_string(),
            success: true,
            started_at: start_utc,
            finished_at: finish_utc,
            duration_ms,
            output: output_path.clone(),
            error: None,
        }),
    };

    let mut state = ctx.state.clone();
    state.build = Some(build_state);
    let state_path = ctx.project.root.join(".typstlab/state.json");
    state.save(&state_path)?;

    // Step 10: Success message
    println!(
        "{} Built '{}' to {} ({}ms)",
        "✓".green().bold(),
        paper_id,
        output_path.display(),
        duration_ms
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    // Integration tests will be in tests/ directory
}
