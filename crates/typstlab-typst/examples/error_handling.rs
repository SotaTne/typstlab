//! Error handling example for typstlab-typst
//!
//! This example demonstrates how to handle various error cases.
//!
//! Run with: cargo run --example error_handling

use std::path::PathBuf;
use typstlab_core::TypstlabError;
use typstlab_typst::{ExecOptions, ResolveOptions, ResolveResult, exec_typst, resolve_typst};

fn main() {
    println!("=== Typstlab-Typst Error Handling Example ===\n");

    // Example 1: Handle version not found
    println!("Example 1: Handling version not found");
    handle_version_not_found();

    println!("\n---\n");

    // Example 2: Handle execution errors
    println!("Example 2: Handling execution errors");
    handle_execution_error();

    println!("\n=== Example completed ===");
}

fn handle_version_not_found() {
    let resolve_opts = ResolveOptions {
        required_version: "99.99.99".to_string(), // Non-existent version
        project_root: PathBuf::from("."),
        force_refresh: false,
    };

    match resolve_typst(resolve_opts) {
        Ok(ResolveResult::NotFound {
            required_version,
            searched_locations,
        }) => {
            println!("Version {} not found (as expected)", required_version);
            println!("Searched locations:");
            for loc in searched_locations {
                println!("  - {}", loc);
            }
        }
        Ok(_) => {
            println!("Unexpectedly found the binary!");
        }
        Err(e) => {
            println!("Error during resolution: {:?}", e);
        }
    }
}

fn handle_execution_error() {
    let exec_opts = ExecOptions {
        project_root: PathBuf::from("."),
        args: vec!["compile".to_string(), "nonexistent.typ".to_string()],
        required_version: "99.99.99".to_string(), // Non-existent version
    };

    match exec_typst(exec_opts) {
        Ok(result) => {
            println!("Execution completed (unexpectedly):");
            println!("  Exit code: {}", result.exit_code);
        }
        Err(TypstlabError::TypstNotResolved { required_version }) => {
            println!(
                "Binary for version {} not resolved (as expected)",
                required_version
            );
            println!("This is the expected error when binary is not found");
        }
        Err(e) => {
            println!("Other error occurred: {:?}", e);
        }
    }
}
