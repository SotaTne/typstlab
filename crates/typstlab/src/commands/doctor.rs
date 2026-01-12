//! Doctor command - environment health check

use crate::context::Context;
use anyhow::Result;
use chrono::Utc;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

/// Doctor command JSON output schema
#[derive(Debug, Serialize, Deserialize)]
struct DoctorOutput {
    schema_version: String,
    project: ProjectInfo,
    timestamp: String,
    checks: Vec<Check>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectInfo {
    name: String,
    root: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Check {
    id: String,
    name: String,
    status: CheckStatus,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum CheckStatus {
    Ok,
    Warning,
    Error,
}

/// Run environment health check
///
/// # Arguments
///
/// * `json` - Output in JSON format if true
/// * `verbose` - Enable verbose output if true
///
/// # Returns
///
/// Always returns Ok(()) - doctor command always exits 0
pub fn run(json: bool, verbose: bool) -> Result<()> {
    // Try to load context, but continue even if it fails
    let ctx_result = Context::new(verbose);

    let mut checks = Vec::new();

    // Check 1: Config validity
    let (project_info, config_check) = match &ctx_result {
        Ok(ctx) => {
            let project_info = ProjectInfo {
                name: ctx.config.project.name.clone(),
                root: ctx.project.root.display().to_string(),
            };

            let check = Check {
                id: "config_valid".to_string(),
                name: "Configuration file".to_string(),
                status: CheckStatus::Ok,
                message: "typstlab.toml is valid".to_string(),
                details: None,
            };

            (project_info, check)
        }
        Err(e) => {
            // Config is invalid, but we still need to provide project info
            let current_dir = env::current_dir().unwrap_or_else(|_| ".".into());
            let project_info = ProjectInfo {
                name: "unknown".to_string(),
                root: current_dir.display().to_string(),
            };

            let check = Check {
                id: "config_valid".to_string(),
                name: "Configuration file".to_string(),
                status: CheckStatus::Error,
                message: format!("Failed to load config: {}", e),
                details: None,
            };

            (project_info, check)
        }
    };

    checks.push(config_check);

    // Check 2: Typst availability (only if context loaded successfully)
    if let Ok(ctx) = &ctx_result {
        let required_version = &ctx.config.typst.version;

        let resolve_options = typstlab_typst::resolve::ResolveOptions {
            required_version: required_version.clone(),
            project_root: ctx.project.root.clone(),
            force_refresh: false,
        };

        match typstlab_typst::resolve::resolve_typst(resolve_options) {
            Ok(result) => {
                use typstlab_typst::resolve::ResolveResult;

                match result {
                    ResolveResult::Cached(info) | ResolveResult::Resolved(info) => {
                        let mut details = HashMap::new();
                        details.insert(
                            "path".to_string(),
                            serde_json::Value::String(info.path.display().to_string()),
                        );
                        details.insert(
                            "version".to_string(),
                            serde_json::Value::String(info.version.clone()),
                        );
                        details.insert(
                            "source".to_string(),
                            serde_json::Value::String(format!("{}", info.source)),
                        );

                        checks.push(Check {
                            id: "typst_available".to_string(),
                            name: "Typst toolchain".to_string(),
                            status: CheckStatus::Ok,
                            message: format!("Typst {} available", info.version),
                            details: Some(details),
                        });
                    }
                    ResolveResult::NotFound {
                        required_version,
                        searched_locations,
                    } => {
                        let mut details = HashMap::new();
                        details.insert(
                            "required_version".to_string(),
                            serde_json::Value::String(required_version.clone()),
                        );
                        details.insert(
                            "searched_locations".to_string(),
                            serde_json::Value::Array(
                                searched_locations
                                    .iter()
                                    .map(|s| serde_json::Value::String(s.clone()))
                                    .collect(),
                            ),
                        );

                        checks.push(Check {
                            id: "typst_available".to_string(),
                            name: "Typst toolchain".to_string(),
                            status: CheckStatus::Error,
                            message: format!(
                                "Typst {} not found in managed cache or system PATH",
                                required_version
                            ),
                            details: Some(details),
                        });
                    }
                }
            }
            Err(e) => {
                let mut details = HashMap::new();
                details.insert(
                    "required_version".to_string(),
                    serde_json::Value::String(required_version.clone()),
                );

                checks.push(Check {
                    id: "typst_available".to_string(),
                    name: "Typst toolchain".to_string(),
                    status: CheckStatus::Error,
                    message: format!("Failed to resolve Typst: {}", e),
                    details: Some(details),
                });
            }
        }
    }

    // Generate output
    let output = DoctorOutput {
        schema_version: "1.0".to_string(),
        project: project_info,
        timestamp: Utc::now().to_rfc3339(),
        checks,
    };

    if json {
        // JSON output
        let json_str = serde_json::to_string_pretty(&output)?;
        println!("{}", json_str);
    } else {
        // Human-readable output
        print_human_readable(&output);
    }

    Ok(())
}

/// Print human-readable output
fn print_human_readable(output: &DoctorOutput) {
    println!("{}", "Environment Health Check".bold());
    println!();

    println!("{}", "Project:".bold());
    println!("  Name: {}", output.project.name);
    println!("  Root: {}", output.project.root);
    println!();

    println!("{}", "Checks:".bold());
    for check in &output.checks {
        let status_str = match check.status {
            CheckStatus::Ok => "✓".green(),
            CheckStatus::Warning => "⚠".yellow(),
            CheckStatus::Error => "✗".red(),
        };

        println!("  {} {}: {}", status_str, check.name.bold(), check.message);

        if let Some(details) = &check.details {
            for (key, value) in details {
                println!("      {}: {}", key, value);
            }
        }
    }

    println!();
    println!("Timestamp: {}", output.timestamp);
}
