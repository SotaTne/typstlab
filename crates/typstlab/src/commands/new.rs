use anyhow::Result;
use colored::Colorize;
use std::env;
use std::path::{Path, PathBuf};
use typstlab_core::project::{create_project, init_project, validate_name};

/// Create a new project
///
/// # Arguments
///
/// * `project_name` - Name of the project (becomes directory name)
/// * `paper_id` - Optional ID of a paper to generate immediately
/// * `verbose` - Enable verbose output if true
pub fn run_new_project(
    project_name: String,
    paper_id: Option<String>,
    verbose: bool,
) -> Result<()> {
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

    // If paper_id provided, create paper
    if let Some(id) = paper_id {
        // We need to change context to the new project dir temporarily to run_new_paper?
        // run_new_paper expects to be inside a project.
        // Or we can manually invoke create_paper logic on the new project path.
        // But run_new_paper uses Context::new() which reads CWD.
        // It's cleaner to implement a helper that takes project path explicitly.
        // Or just change CWD? Changing CWD in library code is risky but CLI command usually ok.
        // Better: refactor run_new_paper to take context or project path.
        // For now, let's create a helper `create_paper_in_project`.

        env::set_current_dir(&project_dir)?;
        crate::commands::paper::run_new(id, verbose)?;
        // Restore CWD? Not strictly necessary for CLI run but good practice if tests reuse process.
        env::set_current_dir(&current_dir)?;
    }

    print_project_structure(verbose);
    print_next_steps_project(&project_name);

    Ok(())
}

/// Initialize a project in an existing directory
///
/// # Arguments
///
/// * `path` - Path to initialize (defaults to CWD)
/// * `paper_id` - Optional ID of a paper to generate immediately
/// * `verbose` - Enable verbose output if true
pub fn run_init(path: Option<String>, paper_id: Option<String>, verbose: bool) -> Result<()> {
    let current_dir = env::current_dir()?;
    let target_path = if let Some(p) = &path {
        Path::new(&p).to_path_buf()
    } else {
        current_dir.clone()
    };

    // Resolve absolute path for clarity
    let target_path = if target_path.is_absolute() {
        target_path
    } else {
        current_dir.join(target_path)
    };

    if verbose {
        println!(
            "{} Initializing project in {}",
            "→".cyan(),
            target_path.display()
        );
    }

    // Ensure target dir exists (if specified via path)
    if !target_path.exists() {
        // If user specified path, maybe create it?
        // `init` usually expects existing dir, OR create it if empty.
        // Let's create it recursively if it doesn't exist (like mkdir -p).
        std::fs::create_dir_all(&target_path)?;
    }

    init_project(&target_path)?;

    println!(
        "{} Initialized project at {}",
        "✓".green().bold(),
        target_path.display()
    );

    if let Some(id) = paper_id {
        let prev_cwd = env::current_dir()?;
        env::set_current_dir(&target_path)?;
        crate::commands::paper::run_new(id, verbose)?;
        env::set_current_dir(prev_cwd)?;
    }

    print_project_structure(verbose);
    // Next steps: stay in current dir if init .
    if target_path == current_dir {
        print_next_steps_init_cwd();
    } else {
        // If path relative, suggest cd
        // We avoid pathdiff dependency here.
        if let Some(p) = &path {
            print_next_steps_project(p);
        } else {
            // Init current dir
            print_next_steps_project("project");
        }
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

#[cfg(test)]
mod tests {
    // Integration tests will be in tests/ directory
}
