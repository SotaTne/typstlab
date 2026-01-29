#[cfg(test)]
mod tests {
    use escargot::CargoBuild;
    use std::sync::LazyLock;

    static XTASK: LazyLock<escargot::CargoRun> = LazyLock::new(|| {
        CargoBuild::new()
            .bin("xtask")
            .run()
            .expect("failed to build xtask")
    });

    #[test]
    fn test_help() {
        let output = XTASK.command().arg("--help").output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Tree-sitter related tasks"));
    }

    #[test]
    fn test_tree_sitter_help() {
        let output = XTASK
            .command()
            .arg("tree-sitter")
            .arg("--help")
            .output()
            .unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Setup tree-sitter environment"));
        assert!(stdout.contains("Generate tree-sitter parsers"));
        assert!(stdout.contains("Verify tree-sitter parsers"));
    }

    #[test]
    fn test_tree_sitter_verify_help() {
        let output = XTASK
            .command()
            .arg("tree-sitter")
            .arg("verify")
            .arg("--help")
            .output()
            .unwrap();

        assert!(output.status.success());
    }
}
