use crate::cli::PaperCommands;
use crate::context::Context;
use anyhow::Result;
use colored::Colorize;
use typstlab_core::paper::Paper;

pub fn run(command: PaperCommands, verbose: bool) -> Result<()> {
    match command {
        PaperCommands::List { json } => run_list(json, verbose),
    }
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
        println!("  typstlab gen paper <paper-id>");
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
