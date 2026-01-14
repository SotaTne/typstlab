//! New command - create project or paper scaffolds

use crate::context::Context;
use anyhow::{Result, bail};
use colored::Colorize;
use std::env;
use typstlab_core::paper::Paper;
use typstlab_core::paper::create_paper;
use typstlab_core::project::{Project, create_project, generate_paper, validate_name};

/// Create a new project
///
/// # Arguments
///
/// * `project_name` - Name of the project (becomes directory name)
/// * `verbose` - Enable verbose output if true
pub fn run_new_project(project_name: String, verbose: bool) -> Result<()> {
    let current_dir = env::current_dir()?;

    if verbose {
        println!(
            "{} Creating project '{}' in {}",
            "→".cyan(),
            project_name,
            current_dir.display()
        );
    }

    // Validate project name (also validated in create_project, but checked here for early failure)
    validate_name(&project_name)?;

    // Create project scaffold
    create_project(&current_dir, &project_name)?;

    let project_dir = current_dir.join(&project_name);

    println!(
        "{} Created project '{}' at {}",
        "✓".green().bold(),
        project_name,
        project_dir.display()
    );

    print_project_structure(verbose);
    print_next_steps_project(&project_name);

    Ok(())
}

/// Print project structure in verbose mode
fn print_project_structure(verbose: bool) {
    if verbose {
        println!("\n{} Project structure:", "→".cyan());
        println!("  - typstlab.toml (project configuration)");
        println!("  - .gitignore");
        println!("  - papers/ (for papers)");
        println!("  - layouts/ (with builtin layouts)");
        println!("  - refs/ (for references)");
        println!("  - dist/ (for build outputs)");
        println!("  - rules/ (for project-level rules)");
        println!("  - .typstlab/ (for state and cache)");
    }
}

/// Print next steps after project creation
fn print_next_steps_project(project_name: &str) {
    println!("\n{} Next steps:", "→".cyan());
    println!("  1. cd {}", project_name);
    println!("  2. typstlab paper new <paper-id>");
    println!("  3. typstlab build --paper <paper-id>");
}

/// Create a new paper in the project
///
/// # Arguments
///
/// * `paper_id` - ID of the paper (becomes directory name)
/// * `verbose` - Enable verbose output if true
pub fn run_new_paper(paper_id: String, verbose: bool) -> Result<()> {
    // Must be inside a project
    let ctx = Context::new(verbose)?;

    if verbose {
        println!("{} Creating paper '{}' in project", "→".cyan(), paper_id);
    }

    // Validate paper ID (also validated in create_paper, but checked here for early failure)
    validate_name(&paper_id)?;

    // Check if paper already exists
    if let Some(_existing) = ctx.project.find_paper(&paper_id) {
        bail!("Paper '{}' already exists", paper_id);
    }

    // Create paper scaffold
    create_paper(&ctx.project, &paper_id)?;

    // Reload project to include newly created paper
    let project = Project::load(ctx.project.root.clone())?;

    // Generate _generated/ directory
    generate_paper(&project, &paper_id)?;

    let paper_dir = project.root.join("papers").join(&paper_id);

    println!(
        "{} Created paper '{}' at {}",
        "✓".green().bold(),
        paper_id,
        paper_dir.display()
    );

    print_paper_structure(verbose);
    print_next_steps_paper(&paper_id);

    Ok(())
}

/// Print paper structure in verbose mode
fn print_paper_structure(verbose: bool) {
    if verbose {
        println!("\n{} Paper structure:", "→".cyan());
        println!("  - paper.toml (paper configuration)");
        println!("  - main.typ (main Typst file)");
        println!("  - sections/ (for paper sections)");
        println!("  - assets/ (for images, etc.)");
        println!("  - rules/ (for paper-specific rules)");
        println!("  - _generated/ (generated layout files)");
    }
}

/// Print next steps after paper creation
fn print_next_steps_paper(paper_id: &str) {
    println!("\n{} Next steps:", "→".cyan());
    println!("  1. Edit papers/{}/paper.toml", paper_id);
    println!("  2. Edit papers/{}/main.typ", paper_id);
    println!("  3. typstlab build --paper {}", paper_id);
}

/// List all papers in the project
///
/// # Arguments
///
/// * `json` - Output as JSON if true
/// * `verbose` - Enable verbose output if true
pub fn run_list_papers(json: bool, verbose: bool) -> Result<()> {
    let ctx = Context::new(verbose)?;
    let papers = ctx.project.papers();

    if json {
        output_papers_json(papers)?;
    } else {
        output_papers_human(papers, verbose);
    }

    Ok(())
}

/// Output papers in JSON format
fn output_papers_json(papers: &[Paper]) -> Result<()> {
    use serde_json::json;

    let papers_json: Vec<_> = papers
        .iter()
        .map(|p| {
            json!({
                "id": p.id(),
                "title": p.config().paper.title,
                "language": p.config().paper.language,
                "date": p.config().paper.date,
                "path": p.root().display().to_string(),
            })
        })
        .collect();

    let output = json!({
        "papers": papers_json,
        "count": papers.len(),
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output papers in human-readable format
fn output_papers_human(papers: &[Paper], verbose: bool) {
    if papers.is_empty() {
        println!("{} No papers found in project", "!".yellow());
        println!("\n{} Create a paper:", "→".cyan());
        println!("  typstlab paper new <paper-id>");
    } else {
        println!("{} Papers in project:", "→".cyan());
        println!();

        for paper in papers {
            println!("  {} {}", "•".cyan(), paper.id());
            println!("    Title: {}", paper.config().paper.title);
            println!("    Language: {}", paper.config().paper.language);
            println!("    Date: {}", paper.config().paper.date);

            if verbose {
                println!("    Path: {}", paper.root().display());
                println!("    Layout: {}", paper.config().layout.name);
            }

            println!();
        }

        println!("{} Total: {} paper(s)", "→".cyan(), papers.len());
    }
}

#[cfg(test)]
mod tests {
    // Integration tests will be in tests/ directory
}
