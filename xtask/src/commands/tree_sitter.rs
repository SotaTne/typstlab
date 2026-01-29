use crate::commands::tree_sitter_verify::{VerifyEnv, VerifyOptions, run_verify};
use anyhow::Result;
use log::info;
use std::path::PathBuf;
use xshell::{Shell, cmd};

pub enum TreeSitterAction {
    Setup,
    Generate,
    Verify { ci: bool, base_branch: String },
}

pub struct TreeSitterCommand {
    pub action: TreeSitterAction,
}

const GRAMMAR_ROOT: &str = "crates/typstlab-lsp/tree-sitter-typstlabtypst";
const TOOLS_ROOT: &str = ".tools";

impl crate::commands::Command for TreeSitterCommand {
    fn run(&self) -> Result<()> {
        let sh = Shell::new()?;
        let repo_root = sh.current_dir(); // Assuming we run from repo root

        // Add .tools/bin to PATH if it exists (for subcommands)
        let _env_guard = {
            let tools_bin = repo_root.join(TOOLS_ROOT).join("bin");
            if tools_bin.exists() {
                let old_path = std::env::var_os("PATH").unwrap_or_default();
                let mut new_path = tools_bin.into_os_string();
                new_path.push(if cfg!(windows) { ";" } else { ":" });
                new_path.push(old_path);
                Some(sh.push_env("PATH", new_path))
            } else {
                None
            }
        };

        match self.action {
            TreeSitterAction::Setup => self.setup(&sh),
            TreeSitterAction::Generate => self.generate(&sh),
            TreeSitterAction::Verify {
                ref ci,
                ref base_branch,
            } => {
                let env = crate::commands::tree_sitter_verify::XshellEnv { sh: &sh };
                let root = PathBuf::from(GRAMMAR_ROOT);
                let abs_root = repo_root.join(&root);
                let src_dir = abs_root.join("src");

                let mut outputs = vec![];
                if src_dir.exists() {
                    let mut stack = vec![src_dir.clone()];
                    while let Some(dir) = stack.pop() {
                        for entry in std::fs::read_dir(dir)? {
                            let entry = entry?;
                            let path = entry.path();
                            if path.is_file() {
                                // Strip abs_root to make it relative to grammar_dir
                                if let Ok(rel_path) = path.strip_prefix(&abs_root) {
                                    outputs.push(root.join(rel_path));
                                }
                            } else if path.is_dir() {
                                stack.push(path);
                            }
                        }
                    }
                }

                let opts = VerifyOptions {
                    ci: *ci,
                    grammar_dir: root.clone(),
                    inputs: vec![root.join("grammar.js")],
                    outputs,
                    base_branch: base_branch.clone(),
                };
                run_verify(&env, &opts)
            }
        }
    }
}

impl TreeSitterCommand {
    pub fn new(action: TreeSitterAction) -> Self {
        Self { action }
    }

    fn setup(&self, sh: &Shell) -> Result<()> {
        info!("Setting up tree-sitter: cargo install --locked tree-sitter-cli --root {TOOLS_ROOT}");
        // We use --root to keep it project-local. Binary will be in .tools/bin/
        cmd!(
            sh,
            "cargo install --locked tree-sitter-cli --root {TOOLS_ROOT}"
        )
        .run()?;
        Ok(())
    }

    fn generate(&self, sh: &Shell) -> Result<()> {
        let env = crate::commands::tree_sitter_verify::XshellEnv { sh };
        env.check_requirements()?;

        let repo_root = sh.current_dir();
        let abs_grammar_dir = repo_root.join(GRAMMAR_ROOT);

        anyhow::ensure!(
            typstlab_core::path::has_absolute_or_rooted_component(&abs_grammar_dir),
            "grammar_dir must be absolute or rooted (caller should join with repo root)"
        );

        info!("Generating tree-sitter parsers in {GRAMMAR_ROOT}...");
        env.tree_sitter_generate(&abs_grammar_dir)?;
        Ok(())
    }
}
