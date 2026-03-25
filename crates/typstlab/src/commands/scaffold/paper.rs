use crate::context::Context;
use anyhow::{Result, bail};
use colored::Colorize;
use typstlab_core::paper::create_paper;
use typstlab_core::project::validate_name;

pub fn run(
    paper_id: String,
    template: Option<String>,
    title: Option<String>,
    verbose: bool,
) -> Result<()> {
    // Must be inside a project
    let ctx = Context::new(verbose)?;

    if verbose {
        println!("{} Creating paper '{}' in project", "→".cyan(), paper_id);
        if let Some(t) = &template {
            println!("  Template: {}", t);
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
    create_paper(
        &ctx.project,
        &paper_id,
        title,
        template,
        Some(|template: &str, path: &std::path::Path| {
            use typstlab_typst::exec::{exec_typst, ExecOptions};

            let exec_opts = ExecOptions {
                project_root: ctx.project.root.clone(),
                args: vec![
                    "init".to_string(),
                    template.to_string(),
                    path.to_string_lossy().to_string(),
                ],
                required_version: ctx.project.config().typst.version.clone(),
            };

            let result = exec_typst(exec_opts)?;
            if result.exit_code != 0 {
                bail!("typst init failed: {}", result.stderr);
            }
            Ok(())
        }),
    )?;



    let paper_dir = ctx.project.root.join("papers").join(&paper_id);

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

    }
}

fn print_next_steps_paper(paper_id: &str) {
    println!("\n{} Next steps:", "→".cyan());
    println!("  1. Edit papers/{}/paper.toml", paper_id);
    println!("  2. Edit papers/{}/main.typ", paper_id);
    println!("  3. typstlab build --paper {}", paper_id);
}
