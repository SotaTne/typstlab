use anyhow::{Result, anyhow};
use std::path::PathBuf;
use typstlab_mcp::serve_stdio;
use typstlab_proto::PROJECT_SETTING_FILE;

pub fn run_stdio(root: PathBuf) -> Result<()> {
    validate_project_root(&root)?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(serve_stdio(root))
        .map_err(|error| anyhow!("MCP stdio server failed: {}", error))
}

fn validate_project_root(root: &std::path::Path) -> Result<()> {
    let config_path = root.join(PROJECT_SETTING_FILE);
    if config_path.is_file() {
        return Ok(());
    }

    Err(anyhow!(
        "MCP project root must contain {}: {}",
        PROJECT_SETTING_FILE,
        root.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::validate_project_root;
    use tempfile::TempDir;
    use typstlab_proto::PROJECT_SETTING_FILE;

    #[test]
    fn test_validate_project_root_requires_project_setting_file() {
        let temp = TempDir::new().unwrap();

        let error = validate_project_root(temp.path()).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("MCP project root must contain typstlab.toml")
        );
    }

    #[test]
    fn test_validate_project_root_accepts_project_setting_file() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join(PROJECT_SETTING_FILE), "").unwrap();

        validate_project_root(temp.path()).unwrap();
    }
}
