//! Doctor command - environment health check

use crate::context::Context;
use anyhow::Result;
use chrono::Utc;
use colored::Colorize;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

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
    #[serde(serialize_with = "serialize_path")]
    root: PathBuf,
}

/// Custom serializer for PathBuf to String
fn serialize_path<S>(path: &Path, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&path.display().to_string())
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

    // Gather all checks
    let project_info = create_project_info(&ctx_result);
    let mut checks = Vec::new();

    checks.push(check_config_validity(&ctx_result));

    if let Ok(ctx) = &ctx_result
        && let Some(typst_check) = check_typst_availability(ctx)
    {
        checks.push(typst_check);
    }

    // Generate output
    let output = DoctorOutput {
        schema_version: "1.0".to_string(),
        project: project_info,
        timestamp: Utc::now().to_rfc3339(),
        checks,
    };

    render_output(&output, json)?;

    Ok(())
}

/// Create ProjectInfo from context result
fn create_project_info(ctx_result: &Result<Context>) -> ProjectInfo {
    match ctx_result {
        Ok(ctx) => ProjectInfo {
            name: ctx.config.project.name.clone(),
            root: ctx.project.root.clone(),
        },
        Err(_) => {
            // Try to get current directory, with detailed error handling
            let root = match env::current_dir() {
                Ok(dir) => dir,
                Err(_) => PathBuf::from("."),
            };

            ProjectInfo {
                name: "unknown".to_string(),
                root,
            }
        }
    }
}

/// Check config validity
fn check_config_validity(ctx_result: &Result<Context>) -> Check {
    match ctx_result {
        Ok(_) => Check {
            id: "config_valid".to_string(),
            name: "Configuration file".to_string(),
            status: CheckStatus::Ok,
            message: "typstlab.toml is valid".to_string(),
            details: None,
        },
        Err(e) => {
            let mut details = HashMap::new();

            // Record current_dir error if it exists
            if let Err(io_err) = env::current_dir() {
                details.insert(
                    "current_dir_error".to_string(),
                    serde_json::Value::String(io_err.to_string()),
                );
            }

            Check {
                id: "config_valid".to_string(),
                name: "Configuration file".to_string(),
                status: CheckStatus::Error,
                message: format!("Failed to load config: {}", e),
                details: Some(details),
            }
        }
    }
}

/// Check Typst toolchain availability
fn check_typst_availability(ctx: &Context) -> Option<Check> {
    let required_version = &ctx.config.typst.version;

    let resolve_options = typstlab_typst::resolve::ResolveOptions {
        required_version: required_version.clone(),
        project_root: ctx.project.root.clone(),
        force_refresh: false,
    };

    match typstlab_typst::resolve::resolve_typst(resolve_options) {
        Ok(result) => {
            use typstlab_typst::resolve::ResolveResult;

            Some(match result {
                ResolveResult::Cached(info) | ResolveResult::Resolved(info) => {
                    create_typst_ok_check(&info)
                }
                ResolveResult::NotFound {
                    required_version,
                    searched_locations,
                } => create_typst_not_found_check(&required_version, &searched_locations),
            })
        }
        Err(e) => Some(create_typst_error_check(required_version, &e)),
    }
}

/// Create Typst OK check
fn create_typst_ok_check(info: &typstlab_typst::info::TypstInfo) -> Check {
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

    Check {
        id: "typst_available".to_string(),
        name: "Typst toolchain".to_string(),
        status: CheckStatus::Ok,
        message: format!("Typst {} available", info.version),
        details: Some(details),
    }
}

/// Create Typst not found check
fn create_typst_not_found_check(required_version: &str, searched_locations: &[String]) -> Check {
    let mut details = HashMap::new();
    details.insert(
        "required_version".to_string(),
        serde_json::Value::String(required_version.to_string()),
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

    Check {
        id: "typst_available".to_string(),
        name: "Typst toolchain".to_string(),
        status: CheckStatus::Error,
        message: format!(
            "Typst {} not found in managed cache or system PATH",
            required_version
        ),
        details: Some(details),
    }
}

/// Create Typst error check
fn create_typst_error_check(
    required_version: &str,
    error: &typstlab_core::error::TypstlabError,
) -> Check {
    let mut details = HashMap::new();
    details.insert(
        "required_version".to_string(),
        serde_json::Value::String(required_version.to_string()),
    );

    Check {
        id: "typst_available".to_string(),
        name: "Typst toolchain".to_string(),
        status: CheckStatus::Error,
        message: format!("Failed to resolve Typst: {}", error),
        details: Some(details),
    }
}

/// Render output in JSON or human-readable format
fn render_output(output: &DoctorOutput, json: bool) -> Result<()> {
    if json {
        let json_str = serde_json::to_string_pretty(output)?;
        println!("{}", json_str);
    } else {
        print_human_readable(output);
    }
    Ok(())
}

/// Print human-readable output
fn print_human_readable(output: &DoctorOutput) {
    println!("{}", "Environment Health Check".bold());
    println!();

    println!("{}", "Project:".bold());
    println!("  Name: {}", output.project.name);
    println!("  Root: {}", output.project.root.display());
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
