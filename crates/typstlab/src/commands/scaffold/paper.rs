use anyhow::{Result, bail};
use colored::Colorize;
use typstlab_core::context::Context;
use typstlab_core::paper::create_paper;
use typstlab_core::project::validate_name;

pub fn run(
    paper_id: String,
    template: Option<String>,
    title: Option<String>,
    verbose: bool,
) -> Result<()> {
    // Must be inside a project
    let ctx = Context::builder().verbose(verbose).build()?;
    run_with_context(&ctx, paper_id, template, title)
}

pub fn run_with_context(
    ctx: &Context,
    paper_id: String,
    template: Option<String>,
    title: Option<String>,
) -> Result<()> {
    let project = ctx
        .project
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not in a typstlab project"))?;

    if ctx.verbose {
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
    if let Some(_existing) = project.find_paper(&paper_id) {
        bail!("Paper '{}' already exists", paper_id);
    }

    // Create paper scaffold
    create_paper(
        project,
        &paper_id,
        title,
        template,
        Some(|template: &str, path: &std::path::Path| {
            use typstlab_typst::exec::{ExecOptions, exec_typst};

            let exec_opts = ExecOptions {
                project_root: project.root.clone(),
                args: vec![
                    "init".to_string(),
                    template.to_string(),
                    path.to_string_lossy().to_string(),
                ],
                required_version: project.config().typst.version.clone(),
            };

            let result = exec_typst(exec_opts)?;
            if result.exit_code != 0 {
                bail!("typst init failed: {}", result.stderr);
            }
            Ok(())
        }),
    )?;

    let paper_dir = project.root.join("papers").join(&paper_id);

    println!(
        "{} Created paper '{}' at {}",
        "✓".green().bold(),
        paper_id,
        paper_dir.display()
    );

    print_paper_structure(ctx.verbose);
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
    }
}

fn print_next_steps_paper(paper_id: &str) {
    println!("\n{} Next steps:", "→".cyan());
    println!("  1. Edit papers/{}/paper.toml", paper_id);
    println!("  2. Edit papers/{}/main.typ", paper_id);
    println!("  3. typstlab build --paper {}", paper_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use typstlab_core::project::init_project;
    use typstlab_testkit::temp_dir_in_workspace;

    #[test]
    fn test_run_paper_basic() {
        let temp = temp_dir_in_workspace();
        let project_dir = temp.path().to_path_buf();
        init_project(&project_dir).unwrap();

        let ctx = Context::builder()
            .env(typstlab_core::context::Environment {
                cache_root: temp.path().join(".cache"),
                cwd: project_dir.clone(),
            })
            .project_root(project_dir.clone())
            .verbose(true)
            .build()
            .unwrap();

        let paper_id = "test-paper".to_string();

        run_with_context(&ctx, paper_id.clone(), None, Some("Test Title".to_string())).unwrap();

        let paper_dir = project_dir.join("papers").join(&paper_id);
        assert!(paper_dir.exists());
        assert!(paper_dir.join("paper.toml").exists());
        assert!(paper_dir.join("main.typ").exists());
    }
}
