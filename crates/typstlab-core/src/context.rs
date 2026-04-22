//! Context for typstlab operations
//!
//! This module provides a centralized context that carries configuration,
//! project information, and environment settings across different crates.

use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use crate::state::State;
use std::path::{Path, PathBuf};

/// Environment settings for typstlab
#[derive(Debug, Clone)]
pub struct Environment {
    /// Root directory for managed cache (e.g., ~/.cache/typstlab)
    pub cache_root: PathBuf,
    /// Current working directory
    pub cwd: PathBuf,
}

impl Environment {
    /// Create environment from variables
    pub fn from_env() -> Self {
        let cache_root = if let Ok(cache_override) = std::env::var("TYPSTLAB_CACHE_DIR") {
            PathBuf::from(cache_override)
        } else {
            match dirs::cache_dir() {
                Some(dir) => dir.join("typstlab"),
                None => std::env::current_dir()
                    .unwrap_or_else(|_| std::env::temp_dir())
                    .join(".tmp")
                    .join(".typstlab-cache"),
            }
        };

        Self {
            cache_root,
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Path to Typst binaries in cache
    pub fn typst_cache_dir(&self) -> PathBuf {
        self.cache_root.join("typst")
    }

    /// Path to documentation in cache
    pub fn docs_cache_dir(&self) -> PathBuf {
        self.cache_root.join("docs")
    }
}

/// Global context containing environment, project, config, and state
pub struct Context {
    pub env: Environment,
    pub project: Option<Project>,
    pub config: Option<Config>,
    pub state: Option<State>,
    pub verbose: bool,
}

impl Context {
    /// Create a new builder for Context
    pub fn builder() -> ContextBuilder {
        ContextBuilder::default()
    }

    /// Create a minimal context for commands that don't require a project (e.g., new)
    pub fn minimal(verbose: bool) -> Self {
        Self {
            env: Environment::from_env(),
            project: None,
            config: None,
            state: None,
            verbose,
        }
    }
    
    /// Get the project root if it exists
    pub fn project_root(&self) -> Option<&Path> {
        self.project.as_ref().map(|p| p.root.as_path())
    }

    /// Get the config or return error
    pub fn config(&self) -> Result<&Config> {
        self.config.as_ref().ok_or_else(|| {
            crate::error::TypstlabError::Generic("Project config not loaded".to_string())
        })
    }
}

#[derive(Default)]
pub struct ContextBuilder {
    env: Option<Environment>,
    project_root: Option<PathBuf>,
    verbose: bool,
}

impl ContextBuilder {
    pub fn env(mut self, env: Environment) -> Self {
        self.env = Some(env);
        self
    }

    pub fn project_root(mut self, root: PathBuf) -> Self {
        self.project_root = Some(root);
        self
    }

    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn build(self) -> Result<Context> {
        let env = self.env.unwrap_or_else(Environment::from_env);
        
        // Find project root if not provided
        let project = if let Some(root) = self.project_root {
            Project::find_root(&root)?
        } else {
            Project::find_root(&env.cwd)?
        };

        if let Some(p) = project {
            let config_path = p.root.join("typstlab.toml");
            let config = Config::from_file(&config_path).ok();
            
            let state_path = p.root.join(".typstlab/state.json");
            let state = Some(State::load_or_empty(&state_path));
            
            Ok(Context {
                env,
                project: Some(p),
                config,
                state,
                verbose: self.verbose,
            })
        } else {
            Ok(Context {
                env,
                project: None,
                config: None,
                state: None,
                verbose: self.verbose,
            })
        }
    }
}
