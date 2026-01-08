//! Basic usage example for typstlab-typst
//!
//! This example demonstrates how to resolve and execute a Typst binary.
//!
//! Run with: cargo run --example basic_usage

use std::path::PathBuf;
use typstlab_typst::{ExecOptions, ResolveOptions, ResolveResult, exec_typst, resolve_typst};

fn main() -> typstlab_core::Result<()> {
    println!("=== Typstlab-Typst Basic Usage Example ===\n");

    // Step 1: Resolve Typst binary
    println!("Step 1: Resolving Typst binary for version 0.17.0...");

    let resolve_opts = ResolveOptions {
        required_version: "0.17.0".to_string(),
        project_root: PathBuf::from("."),
        force_refresh: false,
    };

    match resolve_typst(resolve_opts)? {
        ResolveResult::Cached(info) => {
            println!("✓ Found in cache:");
            println!("  Version: {}", info.version);
            println!("  Path: {:?}", info.path);
            println!("  Source: {:?}", info.source);
        }
        ResolveResult::Resolved(info) => {
            println!("✓ Resolved from {}:", info.source);
            println!("  Version: {}", info.version);
            println!("  Path: {:?}", info.path);
        }
        ResolveResult::NotFound {
            required_version,
            searched_locations,
        } => {
            println!("✗ Version {} not found", required_version);
            println!("  Searched locations:");
            for loc in searched_locations {
                println!("    - {}", loc);
            }
            println!("\nTo install Typst, run:");
            println!("  cargo install typst-cli");
            return Ok(());
        }
    }

    // Step 2: Execute a Typst command
    println!("\nStep 2: Executing typst --version...");

    let exec_opts = ExecOptions {
        project_root: PathBuf::from("."),
        args: vec!["--version".to_string()],
        required_version: "0.17.0".to_string(),
    };

    let exec_result = exec_typst(exec_opts)?;

    println!("✓ Execution completed:");
    println!("  Exit code: {}", exec_result.exit_code);
    println!("  Duration: {}ms", exec_result.duration_ms);
    println!("  Output: {}", exec_result.stdout.trim());

    if !exec_result.stderr.is_empty() {
        println!("  Stderr: {}", exec_result.stderr.trim());
    }

    println!("\n=== Example completed successfully ===");

    Ok(())
}
