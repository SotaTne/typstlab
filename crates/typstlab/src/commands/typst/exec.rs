//! Typst exec command - execute resolved Typst binary with arguments

use anyhow::Result;
use typstlab_core::{TypstlabError, project::Project, state::State};
use typstlab_typst::{ExecOptions, exec_typst};

/// Execute `typstlab typst exec` command
pub fn execute_exec(args: Vec<String>) -> Result<()> {
    // Find project root
    let project = Project::from_current_dir()?;
    let root = &project.root;

    // Get required version from config
    let config = project.config();
    let required_version = &config.typst.version;

    // Load state to verify Typst is resolved
    let state_path = root.join(".typstlab").join("state.json");
    if !state_path.exists() {
        return Err(TypstlabError::Generic(
            "Typst not resolved. Run `typstlab typst link` first.".to_string(),
        )
        .into());
    }

    let state = State::load(&state_path)?;
    if state.typst.is_none() {
        return Err(
            TypstlabError::Generic("No Typst information in state.json".to_string()).into(),
        );
    }

    // Execute typst with the given arguments
    let options = ExecOptions {
        project_root: root.clone(),
        args,
        required_version: required_version.clone(),
    };

    let result = exec_typst(options)?;

    // Print stdout and stderr
    print!("{}", result.stdout);
    eprint!("{}", result.stderr);

    // Exit with the same code as typst
    if result.exit_code != 0 {
        std::process::exit(result.exit_code);
    }

    Ok(())
}
