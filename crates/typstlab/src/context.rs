//! Global context for CLI commands

use anyhow::{Result, anyhow};
use std::env;
use typstlab_core::{config::Config, project::Project, state::State};

/// Global context containing project, config, and state
#[allow(dead_code)]
pub struct Context {
    pub project: Project,
    pub config: Config,
    pub state: State,
    pub verbose: bool,
}

impl Context {
    /// Create a new context by loading project, config, and state
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Not in a typstlab project
    /// - Config file cannot be read or parsed
    /// - State file cannot be read or parsed
    pub fn new(verbose: bool) -> Result<Self> {
        // Find project root
        let current_dir = env::current_dir()?;
        let project = Project::find_root(&current_dir)
            .map_err(|e| anyhow!("Failed to find project: {}", e))?
            .ok_or_else(|| anyhow!("Not in a typstlab project"))?;

        // Load config
        let config_path = project.root.join("typstlab.toml");
        let config = Config::from_file(&config_path)?;

        // Load or create state
        let state_path = project.root.join(".typstlab/state.json");
        let state = State::load_or_empty(&state_path);

        Ok(Self {
            project,
            config,
            state,
            verbose,
        })
    }

    /// Save the current state to disk
    ///
    /// # Errors
    ///
    /// Returns an error if the state file cannot be written
    #[allow(dead_code)]
    pub fn save_state(&self) -> Result<()> {
        let state_path = self.project.root.join(".typstlab/state.json");
        self.state.save(&state_path)?;
        Ok(())
    }
}
