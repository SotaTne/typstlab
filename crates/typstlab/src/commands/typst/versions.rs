use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use typstlab_core::context::Context;

#[derive(Serialize)]
struct VersionEntry {
    version: String,
    source: String,
    path: PathBuf,
    is_current: bool,
}

/// Execute `typstlab typst versions` command
pub fn execute_versions(json: bool) -> Result<()> {
    let ctx = Context::builder().build()?;
    execute_versions_with_context(&ctx, json)
}

pub fn execute_versions_with_context(ctx: &Context, json: bool) -> Result<()> {
    // 1. Identify current version from state
    let current_path = ctx.state.as_ref().and_then(|s| s.typst.as_ref().map(|t| t.resolved_path.clone()));

    let mut entries = Vec::new();

    // 2. Add managed versions from cache
    let cache_dir = ctx.env.typst_cache_dir();
    if cache_dir.exists() {
        for entry in fs::read_dir(cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir()
                && let Some(version) = path.file_name().and_then(|n| n.to_str())
            {
                let binary_path = path.join("typst");
                #[cfg(windows)]
                let binary_path = path.join("typst.exe");

                if binary_path.exists() {
                    let is_current = current_path.as_ref() == Some(&binary_path);
                    entries.push(VersionEntry {
                        version: version.to_string(),
                        source: "managed".to_string(),
                        path: binary_path,
                        is_current,
                    });
                }
            }
        }
    }

    // 3. Add system version from PATH
    if let Ok(system_path) = which::which("typst") {
        // Try to get version by running `typst --version`
        let output = std::process::Command::new(&system_path)
            .arg("--version")
            .output();

        let version = if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .split_whitespace()
                .nth(1)
                .unwrap_or("unknown")
                .to_string()
        } else {
            "unknown".to_string()
        };

        // Avoid duplicates if a managed version is somehow in PATH (unlikely but possible)
        if !entries.iter().any(|e| e.path == system_path) {
            let is_current = current_path.as_ref() == Some(&system_path);
            entries.push(VersionEntry {
                version,
                source: "system".to_string(),
                path: system_path,
                is_current,
            });
        }
    }

    // Sort descending by semver, fallback to string sort
    entries.sort_by(|a, b| {
        let v_a = semver::Version::parse(a.version.strip_prefix('v').unwrap_or(&a.version)).ok();
        let v_b = semver::Version::parse(b.version.strip_prefix('v').unwrap_or(&b.version)).ok();
        match (v_a, v_b) {
            (Some(va), Some(vb)) => vb.cmp(&va),
            _ => b.version.cmp(&a.version),
        }
    });

    if json {
        let output = serde_json::json!({
            "versions": entries
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if entries.is_empty() {
        println!("No Typst versions found.");
    } else {
        println!("{}", "Typst versions:".bold());
        for entry in &entries {
            let current_marker = if entry.is_current {
                "*".green()
            } else {
                " ".into()
            };

            let source_marker = if entry.source == "system" {
                "(local)".dimmed()
            } else {
                "(managed)".dimmed()
            };

            println!(
                "{} {:<10} {:<10} {}",
                current_marker,
                entry.version.cyan(),
                source_marker,
                entry.path.display().to_string().dimmed()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use typstlab_testkit::temp_dir_in_workspace;
    use std::fs;

    #[test]
    fn test_execute_versions_empty() {
        let temp = temp_dir_in_workspace();
        let ctx = Context::builder()
            .env(typstlab_core::context::Environment {
                cache_root: temp.path().join(".cache"),
                cwd: temp.path().to_path_buf(),
            })
            .build()
            .unwrap();

        execute_versions_with_context(&ctx, false).unwrap();
    }

    #[test]
    fn test_execute_versions_with_managed() {
        let temp = temp_dir_in_workspace();
        let cache_root = temp.path().join(".cache");
        let typst_cache = cache_root.join("typst");
        let v_dir = typst_cache.join("0.12.0");
        fs::create_dir_all(&v_dir).unwrap();
        
        let binary_path = v_dir.join("typst");
        fs::write(&binary_path, "dummy").unwrap();

        let ctx = Context::builder()
            .env(typstlab_core::context::Environment {
                cache_root,
                cwd: temp.path().to_path_buf(),
            })
            .build()
            .unwrap();

        execute_versions_with_context(&ctx, false).unwrap();
    }
}

