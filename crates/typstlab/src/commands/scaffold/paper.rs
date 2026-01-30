use crate::context::Context;
use anyhow::{Result, bail};
use colored::Colorize;
use typstlab_core::paper::create_paper;
use typstlab_core::project::{Project, generate_paper, validate_name};

pub fn run(
    paper_id: String,
    layout: Option<String>,
    title: Option<String>,
    verbose: bool,
) -> Result<()> {
    // Must be inside a project
    let ctx = Context::new(verbose)?;

    if verbose {
        println!("{} Creating paper '{}' in project", "→".cyan(), paper_id);
        if let Some(l) = &layout {
            println!("  Theme: {}", l);
        }
        if let Some(t) = &title {
            println!("  Title: {}", t);
        }
    }

    // Validate paper ID
    validate_name(&paper_id)?;

    // Check if paper already exists
    if let Some(_existing) = ctx.project.find_paper(&paper_id) {
        bail!("Paper '{}' already exists", paper_id);
    }

    // Create paper scaffold
    create_paper(&ctx.project, &paper_id, title)?;

    // If layout specified, we'd need to update paper.toml after creation
    // For v0.1 we might just use the default or add layout support to create_paper
    if let Some(_l) = layout {
        // TODO: Update paper.toml with custom theme
        if verbose {
            println!(
                "  ! Custom layout specified, but theme override in paper.toml is not yet automated in v0.1"
            );
        }
    }

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
