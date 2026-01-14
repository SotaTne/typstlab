//! Generate command - create _generated/ directories with rendered templates

use crate::context::Context;
use anyhow::Result;
use colored::Colorize;
use typstlab_core::project::{generate_all_papers, generate_paper};

/// Generate _generated/ directory for papers
///
/// # Arguments
///
/// * `paper_id` - Optional paper ID. If None, generates for all papers
/// * `verbose` - Enable verbose output if true
pub fn run(paper_id: Option<String>, verbose: bool) -> Result<()> {
    let ctx = Context::new(verbose)?;

    match paper_id {
        Some(id) => {
            // Generate single paper
            if verbose {
                println!("{} Generating _generated/ for paper '{}'", "→".cyan(), id);
            }

            generate_paper(&ctx.project, &id)?;

            println!("{} Generated _generated/ for '{}'", "✓".green().bold(), id);
        }
        None => {
            // Generate all papers
            if verbose {
                println!("{} Generating _generated/ for all papers", "→".cyan());
            }

            let generated = generate_all_papers(&ctx.project)?;

            if generated.is_empty() {
                println!("{} No papers found", "!".yellow());
            } else {
                for paper_id in &generated {
                    println!(
                        "{} Generated _generated/ for '{}'",
                        "✓".green().bold(),
                        paper_id
                    );
                }
                println!(
                    "\n{} Generated {} paper(s)",
                    "✓".green().bold(),
                    generated.len()
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // Integration tests will be in tests/ directory
}
