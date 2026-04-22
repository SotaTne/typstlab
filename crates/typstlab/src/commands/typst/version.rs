//! Typst version command - show required and resolved versions

use anyhow::{Result, anyhow};
use serde::Serialize;
use typstlab_core::{
    context::Context,
    state::ResolvedSource,
};

#[derive(Debug, Serialize)]
struct VersionInfo {
    required_version: String,
    resolved_version: Option<String>,
    resolved_source: Option<String>,
    resolved_path: Option<String>,
}

/// Execute `typstlab typst version` command
pub fn execute_version(json: bool) -> Result<()> {
    let ctx = Context::builder().build()?;
    execute_version_with_context(&ctx, json)
}

pub fn execute_version_with_context(ctx: &Context, json: bool) -> Result<()> {
    let config = ctx.config.as_ref().ok_or_else(|| anyhow!("Not in a typstlab project"))?;
    let required_version = &config.typst.version;
    let state = ctx.state.as_ref();

    let version_info = VersionInfo {
        required_version: required_version.clone(),
        resolved_version: state
            .and_then(|s| s.typst.as_ref())
            .map(|t| t.resolved_version.clone()),
        resolved_source: state.and_then(|s| s.typst.as_ref()).map(|t| {
            match t.resolved_source {
                ResolvedSource::Managed => "managed".to_string(),
                ResolvedSource::System => "system".to_string(),
            }
        }),
        resolved_path: state
            .and_then(|s| s.typst.as_ref())
            .map(|t| t.resolved_path.display().to_string()),
    };

    if json {
        // JSON output
        println!("{}", serde_json::to_string_pretty(&version_info)?);
    } else {
        // Human-readable output
        println!("Typst Version Information");
        println!("========================");
        println!();
        println!("Required: {}", version_info.required_version);

        if let Some(resolved_version) = &version_info.resolved_version {
            println!("Resolved: {}", resolved_version);

            if let Some(source) = &version_info.resolved_source {
                println!("Source:   {}", source);
            }

            if let Some(path) = &version_info.resolved_path {
                println!("Path:     {}", path);
            }

            // Check version match
            if resolved_version != &version_info.required_version {
                println!();
                println!("⚠ Warning: Resolved version does not match required version");
                println!("  Run `typstlab setup` to refresh");
            }
        } else {
            println!();
            println!("Status: Not resolved");
            println!("Run `typstlab setup` to resolve Typst");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use typstlab_testkit::temp_dir_in_workspace;
    use typstlab_core::project::init_project;
    use std::fs;

    #[test]
    fn test_execute_version_basic() {
        let temp = temp_dir_in_workspace();
        let project_dir = temp.path().to_path_buf();
        init_project(&project_dir).unwrap();

        // Update config with specific version
        fs::write(project_dir.join("typstlab.toml"), r#"
[project]
name = "test-project"
init_date = "2026-01-15"
[typst]
version = "0.12.0"
"#).unwrap();

        let ctx = Context::builder()
            .project_root(project_dir)
            .build()
            .unwrap();

        execute_version_with_context(&ctx, false).unwrap();
    }
}
