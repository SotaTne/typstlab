use crate::commands::tree_sitter::TreeSitterEnvTrait;
use anyhow::{Context, Result};
use log::info;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct VerifyOptions {
    pub grammar_dir: PathBuf,  // Relative to repo root
    pub outputs: Vec<PathBuf>, // Relative to repo root
}

pub fn run_verify(env: &impl TreeSitterEnvTrait, opts: &VerifyOptions) -> Result<()> {
    let sh = env.sh();
    let repo_root = sh.current_dir();

    // Collect initial state of outputs
    let mut initial_contents = HashMap::new();
    for rel_path in &opts.outputs {
        let abs_path = repo_root.join(rel_path);
        if abs_path.exists() {
            let content = fs::read(&abs_path)
                .with_context(|| format!("Failed to read {}", abs_path.display()))?;
            initial_contents.insert(rel_path.clone(), content);
        }
    }

    info!("Checking output freshness for {:?}", opts.grammar_dir);

    // Always generate in stateless mode
    env.check_requirements()?;
    {
        let abs_grammar_dir = repo_root.join(&opts.grammar_dir);
        env.tree_sitter_generate(&abs_grammar_dir)?;
    }

    // Check for diffs
    let mut dirty_paths = Vec::new();
    for rel_path in &opts.outputs {
        let abs_path = repo_root.join(rel_path);
        if !abs_path.exists() {
            if initial_contents.contains_key(rel_path) {
                dirty_paths.push(rel_path.clone());
            }
            continue;
        }

        let new_content = fs::read(&abs_path)
            .with_context(|| format!("Failed to read after generate: {}", abs_path.display()))?;

        match initial_contents.get(rel_path) {
            Some(old_content) if old_content == &new_content => {}
            _ => {
                dirty_paths.push(rel_path.clone());
            }
        }
    }

    if !dirty_paths.is_empty() {
        anyhow::bail!(
            "Outputs not fresh: after generate, secondary artifacts in {:?} differ from disk.\n\
             Files changed: {:?}\n\
             Please run 'cargo xtask tree-sitter generate' and commit the changes.",
            opts.grammar_dir,
            dirty_paths
        );
    }

    info!("Verify ok: outputs are fresh and match grammar.js");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use typstlab_testkit::temp_dir_in_workspace;
    use xshell::Shell;

    pub struct TestEnv<'a> {
        pub sh: &'a Shell,
        pub mock_generate: Option<Box<dyn Fn() -> Result<()> + 'a>>,
    }

    impl TreeSitterEnvTrait for TestEnv<'_> {
        fn sh(&self) -> &Shell {
            self.sh
        }
        fn tree_sitter_cmd(&self) -> String {
            "tree-sitter".to_string()
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
    }

    fn setup_dummy_repo(path: &Path) {
        fs::write(path.join("readme"), "hello").unwrap();
    }

    #[test]
    fn test_verify_local_clean() {
        let temp = temp_dir_in_workspace();
        let sh = Shell::new().unwrap();
        let _dir = sh.push_dir(temp.path());
        setup_dummy_repo(temp.path());

        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("grammar.js"), "content").unwrap();
        fs::write(temp.path().join("src/parser.c"), "content").unwrap();

        let env = TestEnv {
            sh: &sh,
            mock_generate: None,
        };
        let opts = VerifyOptions {
            grammar_dir: PathBuf::from("."),
            outputs: vec![PathBuf::from("src/parser.c")],
        };

        run_verify(&env, &opts).unwrap();
    }

    #[test]
    fn test_verify_nested_grammar_dir() {
        let temp = temp_dir_in_workspace();
        let sh = Shell::new().unwrap();
        let _dir = sh.push_dir(temp.path());
        setup_dummy_repo(temp.path());

        fs::create_dir_all(temp.path().join("nested/parser/src")).unwrap();
        fs::write(temp.path().join("nested/parser/grammar.js"), "g1").unwrap();
        fs::write(temp.path().join("nested/parser/src/parser.c"), "p1").unwrap();

        let env = TestEnv {
            sh: &sh,
            mock_generate: None,
        };
        let opts = VerifyOptions {
            grammar_dir: PathBuf::from("nested/parser"),
            outputs: vec![PathBuf::from("nested/parser/src/parser.c")],
        };

        run_verify(&env, &opts).unwrap();
    }

    #[test]
    fn test_verify_fails_when_dirty_after_generate() {
        let temp = temp_dir_in_workspace();
        let sh = Shell::new().unwrap();
        let _dir = sh.push_dir(temp.path());
        setup_dummy_repo(temp.path());

        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("grammar.js"), "g1").unwrap();
        fs::write(temp.path().join("src/parser.c"), "old").unwrap();

        let p = temp.path().join("src/parser.c");
        let env = TestEnv {
            sh: &sh,
            mock_generate: Some(Box::new(move || {
                fs::write(&p, "generated but different from disk").unwrap();
                Ok(())
            })),
        };
        let opts = VerifyOptions {
            grammar_dir: PathBuf::from("."),
            outputs: vec![PathBuf::from("src/parser.c")],
        };

        let res = run_verify(&env, &opts);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Outputs not fresh"));
    }
}
