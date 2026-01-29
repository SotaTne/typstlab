use crate::commands::tree_sitter_verify::{VerifyOptions, run_verify};
use anyhow::Result;
use log::info;
use std::path::{Path, PathBuf};
use xshell::{Shell, cmd};

pub enum TreeSitterAction {
    Setup,
    Generate,
    Verify,
    Test,
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
        let _env_guard = prepend_tools_path(&sh, &repo_root);
        let env = TreeSitterEnv { sh: &sh };

        match self.action {
            TreeSitterAction::Setup => self.setup(&env),
            TreeSitterAction::Generate => self.generate(&env),
            TreeSitterAction::Test => self.test(&env),
            TreeSitterAction::Verify => self.verify(&env, &repo_root),
        }
    }
}

impl TreeSitterCommand {
    pub fn new(action: TreeSitterAction) -> Self {
        Self { action }
    }

    fn setup(&self, env: &TreeSitterEnv<'_>) -> Result<()> {
        info!("Setting up tree-sitter: cargo install --locked tree-sitter-cli --root {TOOLS_ROOT}");
        // We use --root to keep it project-local. Binary will be in .tools/bin/
        cmd!(
            env.sh,
            "cargo install --locked tree-sitter-cli --root {TOOLS_ROOT}"
        )
        .run()?;
        Ok(())
    }

    fn generate(&self, env: &TreeSitterEnv<'_>) -> Result<()> {
        env.check_requirements()?;

        let repo_root = env.sh.current_dir();
        let abs_grammar_dir = repo_root.join(GRAMMAR_ROOT);

        info!("Generating tree-sitter parsers in {GRAMMAR_ROOT}...");
        env.tree_sitter_generate(&abs_grammar_dir)?;
        Ok(())
    }

    fn test(&self, env: &TreeSitterEnv<'_>) -> Result<()> {
        env.check_requirements()?;

        let repo_root = env.sh.current_dir();
        let abs_grammar_dir = repo_root.join(GRAMMAR_ROOT);

        anyhow::ensure!(
            typstlab_core::path::has_absolute_or_rooted_component(&abs_grammar_dir),
            "grammar_dir must be absolute or rooted (caller should join with repo root)"
        );

        info!("Testing tree-sitter parsers in {GRAMMAR_ROOT}...");
        let ts = env.tree_sitter_cmd();
        let _dir = env.sh.push_dir(abs_grammar_dir);
        cmd!(env.sh, "{ts} test").run()?;
        Ok(())
    }

    fn verify(&self, env: &TreeSitterEnv<'_>, repo_root: &Path) -> Result<()> {
        let grammar_root = PathBuf::from(GRAMMAR_ROOT);
        let abs_root = repo_root.join(&grammar_root);

        let outputs = collect_outputs(&abs_root, &grammar_root)?;
        let opts = VerifyOptions {
            grammar_dir: grammar_root.clone(),
            outputs,
        };
        run_verify(env, &opts)
    }
}

pub trait TreeSitterEnvTrait {
    fn sh(&self) -> &Shell;
    fn tree_sitter_cmd(&self) -> String;
    fn check_requirements(&self) -> Result<()>;
    fn tree_sitter_generate(&self, grammar_dir: &Path) -> Result<()>;
}

pub struct TreeSitterEnv<'a> {
    pub sh: &'a Shell,
}

impl TreeSitterEnvTrait for TreeSitterEnv<'_> {
    fn sh(&self) -> &Shell {
        self.sh
    }

    fn tree_sitter_cmd(&self) -> String {
        "tree-sitter".to_string()
    }

    fn check_requirements(&self) -> Result<()> {
        let ts = self.tree_sitter_cmd();
        if cmd!(self.sh, "{ts} --version").quiet().run().is_err() {
            anyhow::bail!(
                "tree-sitter CLI not found; please install it or run 'cargo xtask tree-sitter setup'"
            );
        }
        if cmd!(self.sh, "node --version").quiet().run().is_err() {
            anyhow::bail!(
                "node not found in PATH; required for tree-sitter generate. Please install Node.js (v18+ recommended)."
            );
        }
        Ok(())
    }

    fn tree_sitter_generate(&self, grammar_dir: &Path) -> Result<()> {
        anyhow::ensure!(
            typstlab_core::path::has_absolute_or_rooted_component(grammar_dir),
            "grammar_dir must be absolute or rooted (caller should join with repo root)"
        );
        let ts = self.tree_sitter_cmd();
        let _dir = self.sh.push_dir(grammar_dir);
        cmd!(self.sh, "{ts} generate").run()?;
        Ok(())
    }
}

fn prepend_tools_path<'a>(sh: &'a Shell, repo_root: &Path) -> Option<xshell::PushEnv<'a>> {
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
}

fn collect_outputs(abs_root: &Path, rel_root: &Path) -> Result<Vec<PathBuf>> {
    let src_dir = abs_root.join("src");
    let mut outputs = Vec::new();

    if !src_dir.exists() {
        return Ok(outputs);
    }

    let mut stack = vec![src_dir];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Ok(rel_path) = path.strip_prefix(abs_root) {
                    outputs.push(rel_root.join(rel_path));
                }
            } else if path.is_dir() {
                stack.push(path);
            }
        }
    }

    Ok(outputs)
}
