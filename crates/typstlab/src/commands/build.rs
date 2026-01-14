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
pub fn run(paper_id: String, full: bool, verbose: bool) -> Result<()> {
    let ctx = Context::new(verbose)?;

    // Step 1: Find paper
    if verbose {
        println!("{} Finding paper '{}'", "→".cyan(), paper_id);
    }

    let paper = ctx
        .project
        .find_paper(&paper_id)
        .ok_or_else(|| anyhow::anyhow!("Paper '{}' not found", paper_id))?;

    // Step 2: Check main file exists
    if verbose {
        println!("{} Checking main file", "→".cyan());
    }

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
        if verbose {
            println!(
                "{} Checking root directory '{}'",
                "→".cyan(),
                root_dir.display()
            );
        }

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
                println!("{} Forcing regeneration of _generated/", "→".cyan());
            } else {
                println!("{} Generating _generated/ (not found)", "→".cyan());
            }
        }

        generate_paper(&ctx.project, &paper_id)?;

        if verbose {
            println!("{} Generated _generated/", "✓".green().bold());
        }
    } else if verbose {
        println!("{} Using existing _generated/", "→".cyan());
    }

    // Step 5: Create dist/<paper_id>/ directory
    let dist_dir = ctx.project.root.join("dist").join(&paper_id);
    fs::create_dir_all(&dist_dir)?;

    if verbose {
        println!(
            "{} Created dist directory '{}'",
            "→".cyan(),
            dist_dir.display()
        );
    }

    // Step 6: Build Typst compile command
    let output_filename = format!("{}.pdf", paper.config().output.name);
    let output_path = dist_dir.join(&output_filename);

    let mut args = vec!["compile".to_string()];

    // Add --root option if specified
    if let Some(root_dir) = paper.typst_root_dir() {
        args.push("--root".to_string());
        args.push(root_dir.display().to_string());

        if verbose {
            println!("{} Using --root {}", "→".cyan(), root_dir.display());
        }
    }

    args.push(main_file_path.display().to_string());
    args.push(output_path.display().to_string());

    // Step 7: Execute Typst compile
    if verbose {
        println!("{} Compiling paper to PDF", "→".cyan());
        println!("  Command: typst {}", args.join(" "));
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
        eprintln!("{} Build failed", "✗".red().bold());
        eprintln!("Exit code: {}", exec_result.exit_code);
        if !exec_result.stderr.is_empty() {
            eprintln!("Stderr:\n{}", exec_result.stderr);
        }
        if !exec_result.stdout.is_empty() {
            eprintln!("Stdout:\n{}", exec_result.stdout);
        }

        // Update state with failure
        let build_state = BuildState {
            last: Some(LastBuild {
                paper: paper_id.clone(),
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
            "Typst compilation failed with exit code {}",
            exec_result.exit_code
        );
    }

    // Step 9: Update state.json with success
    if verbose {
        println!("{} Updating state.json", "→".cyan());
    }

    let build_state = BuildState {
        last: Some(LastBuild {
            paper: paper_id.clone(),
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
