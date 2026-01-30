use crate::cli::PaperCommands;
use crate::context::Context;
use anyhow::{Result, bail};
use colored::Colorize;
use typstlab_core::paper::{Paper, create_paper};
use typstlab_core::project::{Project, generate_paper, validate_name};

pub fn run(command: PaperCommands, verbose: bool) -> Result<()> {
    match command {
        PaperCommands::New { id } => run_new(id, verbose),
        PaperCommands::List { json } => run_list(json, verbose),
    }
}

pub fn run_new(paper_id: String, verbose: bool) -> Result<()> {
    // Must be inside a project
    let ctx = Context::new(verbose)?;

    if verbose {
        println!("{} Creating paper '{}' in project", "→".cyan(), paper_id);
    }

    // Validate paper ID
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

fn run_list(json: bool, verbose: bool) -> Result<()> {
    let ctx = Context::new(verbose)?;
    let papers = ctx.project.papers();

    if json {
        output_papers_json(papers)?;
    } else {
        output_papers_human(papers, verbose);
    }

    Ok(())
}

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

fn print_next_steps_paper(paper_id: &str) {
    println!("\n{} Next steps:", "→".cyan());
    println!("  1. Edit papers/{}/paper.toml", paper_id);
    println!("  2. Edit papers/{}/main.typ", paper_id);
    println!("  3. typstlab build --paper {}", paper_id);
}

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
                println!("    Theme: {}", paper.config().layout.theme);
            }

            println!();
        }

        println!("{} Total: {} paper(s)", "→".cyan(), papers.len());
    }
}
