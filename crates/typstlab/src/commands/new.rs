use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use typstlab_core::config::Config;
use typstlab_core::context::Context;
use typstlab_core::path::has_absolute_or_rooted_component;
use typstlab_core::project::{create_project, init_project, validate_name};

/// Create a new project
pub fn run_new_project(
    project_name: String,
    paper_id: Option<String>,
    verbose: bool,
) -> Result<()> {
    let ctx = Context::minimal(verbose);
    run_new_project_with_context(&ctx, project_name, paper_id)
}

pub fn run_new_project_with_context(
    ctx: &Context,
    project_name: String,
    paper_id: Option<String>,
) -> Result<()> {
    let current_dir = &ctx.env.cwd;

    if ctx.verbose {
        println!(
            "{} Creating project '{}' in {}",
            "→".cyan(),
            project_name,
            current_dir.display()
        );
    }

    // Validate project name
    validate_name(&project_name)?;

    // Create project scaffold
    create_project(current_dir, &project_name)?;

    let project_dir = current_dir.join(&project_name);

    println!(
        "{} Created project '{}' at {}",
        "✓".green().bold(),
        project_name,
        project_dir.display()
    );

    // If paper_id provided, create paper
    if let Some(id) = paper_id {
        // Use Builder to create a context for the new project
        let project_ctx = Context::builder()
            .env(ctx.env.clone())
            .project_root(project_dir.clone())
            .verbose(ctx.verbose)
            .build()?;
        crate::commands::scaffold::paper::run_with_context(&project_ctx, id, None, None)?;
    }

    // Auto-sync documentation
    if ctx.verbose {
        println!("→ Syncing documentation...");
    }
    let config = Config::from_file(project_dir.join("typstlab.toml"))?;
    let docs_target = project_dir.join(".typstlab/kb/typst/docs");
    typstlab_typst::docs::sync_docs(&config.typst.version, &docs_target, ctx.verbose)?;

    print_project_structure(ctx.verbose);
    print_next_steps_project(&project_name);

    Ok(())
}

/// Initialize a project in an existing directory
pub fn run_init(path: Option<String>, paper_id: Option<String>, verbose: bool) -> Result<()> {
    let ctx = Context::minimal(verbose);
    run_init_with_context(&ctx, path, paper_id)
}

pub fn run_init_with_context(
    ctx: &Context,
    path: Option<String>,
    paper_id: Option<String>,
) -> Result<()> {
    let current_dir = &ctx.env.cwd;
    let target_path = if let Some(p) = &path {
        typstlab_core::path::expand_tilde(Path::new(&p))
    } else {
        current_dir.clone()
    };

    // Resolve absolute path for clarity
    let target_path = if has_absolute_or_rooted_component(&target_path) {
        target_path
    } else {
        current_dir.join(target_path)
    };

    if ctx.verbose {
        println!(
            "{} Initializing project in {}",
            "→".cyan(),
            target_path.display()
        );
    }

    // Ensure target dir exists
    if !target_path.exists() {
        std::fs::create_dir_all(&target_path)?;
    }

    init_project(&target_path)?;

    println!(
        "{} Initialized project at {}",
        "✓".green().bold(),
        target_path.display()
    );

    if let Some(id) = paper_id {
        let project_ctx = Context::builder()
            .env(ctx.env.clone())
            .project_root(target_path.clone())
            .verbose(ctx.verbose)
            .build()?;
        crate::commands::scaffold::paper::run_with_context(&project_ctx, id, None, None)?;
    }

    // Auto-sync documentation
    if ctx.verbose {
        println!("→ Syncing documentation...");
    }
    let config = Config::from_file(target_path.join("typstlab.toml"))?;
    let docs_target = target_path.join(".typstlab/kb/typst/docs");
    typstlab_typst::docs::sync_docs(&config.typst.version, &docs_target, ctx.verbose)?;

    print_project_structure(ctx.verbose);
    
    if target_path == *current_dir {
        print_next_steps_init_cwd();
    } else if let Some(p) = &path {
        print_next_steps_project(p);
    } else {
        print_next_steps_project("project");
    }

    Ok(())
}


/// Print next steps when initialized in CWD
fn print_next_steps_init_cwd() {
    println!("\n{} Next steps:", "→".cyan());
    println!("  1. typstlab gen paper <paper-id>");
    println!("  2. typstlab build --paper <paper-id>");
}

/// Print project structure in verbose mode
fn print_project_structure(verbose: bool) {
    if verbose {
        println!("\n{} Project structure:", "→".cyan());
        println!("  - typstlab.toml (project configuration)");
        println!("  - .gitignore");
        println!("  - papers/ (for papers)");
        println!("  - templates/ (with builtin templates)");
        println!("  - refs/ (for references)");
        println!("  - dist/ (for build outputs)");
        println!("  - .typstlab/ (for state and cache)");
    }
}

/// Print next steps after project creation
fn print_next_steps_project(project_name: &str) {
    println!("\n{} Next steps:", "→".cyan());
    println!("  1. cd {}", project_name);
    println!("  2. typstlab gen paper <paper-id>");
    println!("  3. typstlab build --paper <paper-id>");
}

#[cfg(test)]
mod tests {
    use super::*;
    use typstlab_testkit::temp_dir_in_workspace;
    use std::fs;

    #[test]
    fn test_run_new_project_basic() {
        let temp = temp_dir_in_workspace();
        let ctx = Context::builder()
            .env(typstlab_core::context::Environment {
                cache_root: temp.path().join(".cache"),
                cwd: temp.path().to_path_buf(),
            })
            .verbose(true)
            .build()
            .unwrap();

        let project_name = "test-project".to_string();
        
        // Mock sync_docs behavior by not actually calling it during tests
        // or just let it fail if it needs network but we are in a disconnected env.
        // Actually, let's just test project creation part first.
        
        run_new_project_with_context(&ctx, project_name.clone(), None).unwrap();

        let project_dir = temp.path().join(&project_name);
        assert!(project_dir.exists());
        assert!(project_dir.join("typstlab.toml").exists());
        assert!(project_dir.join("papers").exists());
    }

    #[test]
    fn test_run_init_basic() {
        let temp = temp_dir_in_workspace();
        let project_dir = temp.path().join("init-me");
        fs::create_dir_all(&project_dir).unwrap();

        let ctx = Context::builder()
            .env(typstlab_core::context::Environment {
                cache_root: temp.path().join(".cache"),
                cwd: temp.path().to_path_buf(),
            })
            .verbose(true)
            .build()
            .unwrap();

        run_init_with_context(&ctx, Some("init-me".to_string()), None).unwrap();

        assert!(project_dir.join("typstlab.toml").exists());
        assert!(project_dir.join("papers").exists());
    }
}
