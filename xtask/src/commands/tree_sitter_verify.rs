use anyhow::Result;
use log::info;
use std::path::{Path, PathBuf};
use xshell::{Shell, cmd};

pub trait VerifyEnv {
    fn git_diff_names(&self, base_commit: &str) -> Result<Vec<String>>;
    fn is_dirty(&self, paths: &[PathBuf]) -> Result<bool>;
    fn check_requirements(&self) -> Result<()>;
    fn tree_sitter_generate(&self, grammar_dir: &Path) -> Result<()>;
    fn get_base_commit(&self, base_branch: &str) -> Result<String>;
    fn get_repo_root(&self) -> Result<PathBuf>;
}

pub struct XshellEnv<'a> {
    pub sh: &'a Shell,
}

impl XshellEnv<'_> {
    fn tree_sitter_cmd(&self) -> String {
        let bin_name = if cfg!(windows) {
            "tree-sitter.exe"
        } else {
            "tree-sitter"
        };
        // Use get_repo_root to ensure we find .tools regardless of current push_dir state
        let root = self
            .get_repo_root()
            .unwrap_or_else(|_| self.sh.current_dir());
        let local_ts = root.join(".tools/bin").join(bin_name);

        if local_ts.exists() {
            local_ts.to_string_lossy().to_string()
        } else {
            bin_name.to_string()
        }
    }
}

impl VerifyEnv for XshellEnv<'_> {
    fn git_diff_names(&self, base_commit: &str) -> Result<Vec<String>> {
        let output = cmd!(self.sh, "git diff --name-only {base_commit}..HEAD").read()?;
        Ok(output.lines().map(|s| s.to_string()).collect())
    }

    fn is_dirty(&self, paths: &[PathBuf]) -> Result<bool> {
        if paths.is_empty() {
            return Ok(false);
        }
        let root = self.get_repo_root()?;
        let abs_paths: Vec<_> = paths.iter().map(|p| root.join(p)).collect();
        let output = cmd!(self.sh, "git status --porcelain -- {abs_paths...}").read()?;
        Ok(!output.trim().is_empty())
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

    fn get_base_commit(&self, base_branch: &str) -> Result<String> {
        let base = cmd!(self.sh, "git merge-base {base_branch} HEAD").read();
        match base {
            Ok(b) => Ok(b.trim().to_string()),
            Err(_) => {
                anyhow::bail!("CI mode requires BASE commit (merge-base {base_branch} HEAD)")
            }
        }
    }

    fn get_repo_root(&self) -> Result<PathBuf> {
        let out = cmd!(self.sh, "git rev-parse --show-toplevel").read()?;
        Ok(PathBuf::from(out.trim()))
    }
}

pub struct VerifyOptions {
    pub ci: bool,
    pub grammar_dir: PathBuf,  // Relative to repo root
    pub inputs: Vec<PathBuf>,  // Relative to repo root
    pub outputs: Vec<PathBuf>, // Relative to repo root
    pub base_branch: String,
}

pub fn run_verify(env: &impl VerifyEnv, opts: &VerifyOptions) -> Result<()> {
    let repo_root = env.get_repo_root()?;

    // Filter existing outputs
    let outputs: Vec<_> = opts
        .outputs
        .iter()
        .filter(|p| repo_root.join(p).exists())
        .cloned()
        .collect();

    if opts.ci {
        verify_ci(env, opts, &outputs)
    } else {
        verify_local(env, opts, &outputs)
    }
}

fn verify_ci(env: &impl VerifyEnv, opts: &VerifyOptions, outputs: &[PathBuf]) -> Result<()> {
    info!("CI Mode: Strictly checking commit coherence and output freshness");
    let base = env.get_base_commit(&opts.base_branch)?;

    let changed_files = env.git_diff_names(&base)?;
    let inputs_changed_in_commit = opts
        .inputs
        .iter()
        .any(|i| changed_files.iter().any(|f| Path::new(f) == i));
    let outputs_changed_in_commit = outputs
        .iter()
        .any(|o| changed_files.iter().any(|f| Path::new(f) == o));

    if !inputs_changed_in_commit && outputs_changed_in_commit {
        anyhow::bail!(
            "Invalid commit: outputs changed but inputs ({:?}) did not ({base}..HEAD)",
            opts.inputs
        );
    }

    if inputs_changed_in_commit {
        env.check_requirements()?;
        {
            let repo_root = env.get_repo_root()?;
            let abs_grammar_dir = repo_root.join(&opts.grammar_dir);
            env.tree_sitter_generate(&abs_grammar_dir)?;
        }

        if env.is_dirty(outputs)? {
            anyhow::bail!(
                "Outputs not fresh: after generate, outputs have diffs. Commit generated files."
            );
        }
        info!("CI verify ok: commit coherent + outputs fresh");
    } else {
        if env.is_dirty(outputs)? {
            anyhow::bail!("Working tree dirty: outputs differ from HEAD (or untracked)");
        }
        info!("CI verify ok: no grammar changes; outputs clean");
    }
    Ok(())
}

fn verify_local(env: &impl VerifyEnv, opts: &VerifyOptions, outputs: &[PathBuf]) -> Result<()> {
    info!("Local Mode: Checking freshness");
    let inputs_changed = env.is_dirty(&opts.inputs)?;

    if inputs_changed {
        env.check_requirements()?;
        {
            let repo_root = env.get_repo_root()?;
            let abs_grammar_dir = repo_root.join(&opts.grammar_dir);
            env.tree_sitter_generate(&abs_grammar_dir)?;
        }

        if env.is_dirty(outputs)? {
            anyhow::bail!(
                "Outputs not fresh: after generate, outputs have diffs. Commit/update outputs."
            );
        }
        info!("Local verify ok: generate succeeded and outputs are fresh");
    } else {
        if env.is_dirty(outputs)? {
            anyhow::bail!(
                "Outputs differ from HEAD (without grammar change). Run generate or revert outputs."
            );
        }
        info!("Local verify ok: outputs clean");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use typstlab_testkit::temp_dir_in_workspace;

    struct TestEnv<'a> {
        sh: &'a Shell,
        mock_generate: Option<Box<dyn Fn() -> Result<()> + 'a>>,
    }

    impl VerifyEnv for TestEnv<'_> {
        fn git_diff_names(&self, base_commit: &str) -> Result<Vec<String>> {
            let output = cmd!(self.sh, "git diff --name-only {base_commit}..HEAD").read()?;
            Ok(output.lines().map(|s| s.to_string()).collect())
        }

        fn is_dirty(&self, paths: &[PathBuf]) -> Result<bool> {
            if paths.is_empty() {
                return Ok(false);
            }
            let root = self.get_repo_root()?;
            let abs_paths: Vec<_> = paths.iter().map(|p| root.join(p)).collect();
            let output = cmd!(self.sh, "git status --porcelain -- {abs_paths...}").read()?;
            Ok(!output.trim().is_empty())
        }

        fn check_requirements(&self) -> Result<()> {
            Ok(())
        }

        fn tree_sitter_generate(&self, _grammar_dir: &Path) -> Result<()> {
            if let Some(ref mock) = self.mock_generate {
                mock()
            } else {
                Ok(())
            }
        }

        fn get_base_commit(&self, base_branch: &str) -> Result<String> {
            let base = cmd!(self.sh, "git merge-base {base_branch} HEAD").read();
            match base {
                Ok(b) => Ok(b.trim().to_string()),
                Err(_) => anyhow::bail!("error"),
            }
        }

        fn get_repo_root(&self) -> Result<PathBuf> {
            let out = cmd!(self.sh, "git rev-parse --show-toplevel").read()?;
            Ok(PathBuf::from(out.trim()))
        }
    }

    fn setup_git_repo(sh: &Shell, path: &Path) {
        cmd!(sh, "git init").quiet().run().unwrap();
        cmd!(sh, "git config user.email 'test@example.com'")
            .quiet()
            .run()
            .unwrap();
        cmd!(sh, "git config user.name 'Test User'")
            .quiet()
            .run()
            .unwrap();
        fs::write(path.join("readme"), "hello").unwrap();
        cmd!(sh, "git add .").quiet().run().unwrap();
        cmd!(sh, "git commit -m 'initial'").quiet().run().unwrap();
        cmd!(sh, "git checkout -B main").quiet().run().unwrap();
    }

    #[test]
    fn test_verify_local_clean() {
        let temp = temp_dir_in_workspace();
        let sh = Shell::new().unwrap();
        let _dir = sh.push_dir(temp.path());
        setup_git_repo(&sh, temp.path());

        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("grammar.js"), "content").unwrap();
        fs::write(temp.path().join("src/parser.c"), "content").unwrap();

        cmd!(sh, "git add .").quiet().run().unwrap();
        cmd!(sh, "git commit -m 'add grammar'")
            .quiet()
            .run()
            .unwrap();

        let env = TestEnv {
            sh: &sh,
            mock_generate: None,
        };
        let opts = VerifyOptions {
            ci: false,
            grammar_dir: PathBuf::from("."),
            inputs: vec![PathBuf::from("grammar.js")],
            outputs: vec![PathBuf::from("src/parser.c")],
            base_branch: "main".to_string(),
        };

        run_verify(&env, &opts).unwrap();
    }

    #[test]
    fn test_verify_nested_grammar_dir() {
        let temp = temp_dir_in_workspace();
        let sh = Shell::new().unwrap();
        let _dir = sh.push_dir(temp.path());
        setup_git_repo(&sh, temp.path());

        fs::create_dir_all(temp.path().join("nested/parser/src")).unwrap();
        fs::write(temp.path().join("nested/parser/grammar.js"), "g1").unwrap();
        fs::write(temp.path().join("nested/parser/src/parser.c"), "p1").unwrap();

        cmd!(sh, "git add .").quiet().run().unwrap();
        cmd!(sh, "git commit -m 'base'").quiet().run().unwrap();

        let env = TestEnv {
            sh: &sh,
            mock_generate: None,
        };
        let opts = VerifyOptions {
            ci: false,
            grammar_dir: PathBuf::from("nested/parser"),
            inputs: vec![PathBuf::from("nested/parser/grammar.js")],
            outputs: vec![PathBuf::from("nested/parser/src/parser.c")],
            base_branch: "main".to_string(),
        };

        run_verify(&env, &opts).unwrap();
    }

    #[test]
    fn test_verify_ci_incoherent() {
        let temp = temp_dir_in_workspace();
        let sh = Shell::new().unwrap();
        let _dir = sh.push_dir(temp.path());
        setup_git_repo(&sh, temp.path());

        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("grammar.js"), "g1").unwrap();
        fs::write(temp.path().join("src/parser.c"), "p1").unwrap();
        cmd!(sh, "git add .").quiet().run().unwrap();
        cmd!(sh, "git commit -m 'base'").quiet().run().unwrap();

        cmd!(sh, "git checkout -b feature").quiet().run().unwrap();
        fs::write(temp.path().join("src/parser.c"), "p2").unwrap();
        cmd!(sh, "git add .").quiet().run().unwrap();
        cmd!(sh, "git commit -m 'incoherent'")
            .quiet()
            .run()
            .unwrap();

        let env = TestEnv {
            sh: &sh,
            mock_generate: None,
        };
        let opts = VerifyOptions {
            ci: true,
            grammar_dir: PathBuf::from("."),
            inputs: vec![PathBuf::from("grammar.js")],
            outputs: vec![PathBuf::from("src/parser.c")],
            base_branch: "main".to_string(),
        };

        let res = run_verify(&env, &opts);
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Invalid commit: outputs changed but inputs")
        );
    }

    #[test]
    fn test_verify_ci_stale_output() {
        let temp = temp_dir_in_workspace();
        let sh = Shell::new().unwrap();
        let _dir = sh.push_dir(temp.path());
        setup_git_repo(&sh, temp.path());

        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("grammar.js"), "g1").unwrap();
        fs::write(temp.path().join("src/parser.c"), "generated").unwrap();
        cmd!(sh, "git add .").quiet().run().unwrap();
        cmd!(sh, "git commit -m 'base'").quiet().run().unwrap();

        cmd!(sh, "git checkout -b feature").quiet().run().unwrap();
        fs::write(temp.path().join("grammar.js"), "g2").unwrap();
        cmd!(sh, "git add .").quiet().run().unwrap();
        cmd!(sh, "git commit -m 'stale input'")
            .quiet()
            .run()
            .unwrap();

        // When generate is called, it should change the output to something else
        let p = temp.path().join("src/parser.c");
        let env = TestEnv {
            sh: &sh,
            mock_generate: Some(Box::new(move || {
                fs::write(&p, "new output").unwrap();
                Ok(())
            })),
        };

        let opts = VerifyOptions {
            ci: true,
            grammar_dir: PathBuf::from("."),
            inputs: vec![PathBuf::from("grammar.js")],
            outputs: vec![PathBuf::from("src/parser.c")],
            base_branch: "main".to_string(),
        };

        let res = run_verify(&env, &opts);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Outputs not fresh"));
    }
}
