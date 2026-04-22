//! Doctor command - environment health check

use typstlab_core::context::Context;
use anyhow::Result;
use chrono::Utc;
use colored::Colorize;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use typstlab_core::status::schema::{Check, CheckStatus};

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
    let ctx_result = Context::builder().verbose(verbose).build().map_err(anyhow::Error::from);

    // Gather all checks
    let project_info = create_project_info(&ctx_result);
    let mut checks = Vec::new();

    checks.push(check_config_validity(&ctx_result));

    if let Ok(ctx) = &ctx_result {
        if let Some(typst_check) = check_typst_availability(ctx) {
            checks.push(typst_check);
        }
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
        Ok(ctx) if ctx.project.is_some() => ProjectInfo {
            name: ctx.config.as_ref().unwrap().project.name.clone(),
            root: ctx.project.as_ref().unwrap().root.clone(),
        },
        _ => {
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
        Ok(ctx) => {
            if ctx.config.is_some() {
                Check {
                    id: "config_valid".to_string(),
                    name: "Configuration validity".to_string(),
                    status: CheckStatus::Pass,
                    message: "typstlab.toml is valid".to_string(),
                    details: None,
                }
            } else {
                Check {
                    id: "config_missing".to_string(),
                    name: "Configuration validity".to_string(),
                    status: CheckStatus::Error,
                    message: "typstlab.toml not found or invalid".to_string(),
                    details: None,
                }
            }
        }
        Err(e) => Check {
            id: "config_error".to_string(),
            name: "Configuration validity".to_string(),
            status: CheckStatus::Error,
            message: format!("Error loading context: {}", e),
            details: None,
        },
    }
}

/// Check Typst availability
fn check_typst_availability(ctx: &Context) -> Option<Check> {
    use typstlab_typst::resolve::{ResolveOptions, resolve_typst, ResolveResult};

    let config = ctx.config.as_ref()?;
    
    let options = ResolveOptions {
        required_version: config.typst.version.clone(),
        project_root: ctx.project.as_ref().unwrap().root.clone(),
        force_refresh: false,
    };

    let result = resolve_typst(options).ok()?;

    match result {
        ResolveResult::Cached(info) | ResolveResult::Resolved(info) => Some(Check {
            id: "typst_available".to_string(),
            name: "Typst toolchain".to_string(),
            status: CheckStatus::Pass,
            message: format!("Typst {} available", info.version),
            details: Some(HashMap::from([
                ("version".to_string(), info.version.into()),
                ("path".to_string(), info.path.display().to_string().into()),
            ])),
        }),
        ResolveResult::NotFound {
            required_version,
            searched_locations,
        } => Some(Check {
            id: "typst_available".to_string(),
            name: "Typst toolchain".to_string(),
            status: CheckStatus::Error,
            message: format!("Typst {} not found", required_version),
            details: Some(HashMap::from([(
                "searched_locations".to_string(),
                searched_locations.into(),
            )])),
        }),
    }
}

/// Render output to console or as JSON
fn render_output(output: &DoctorOutput, json: bool) -> Result<()> {
    if json {
        let json_str = serde_json::to_string_pretty(output)?;
        println!("{}", json_str);
    } else {
        println!("{} typstlab doctor report", "⚕".bold());
        println!("Project: {} ({})", output.project.name.cyan(), output.project.root.display());
        println!("Timestamp: {}\n", output.timestamp);

        println!("Checks:");
        for check in &output.checks {
            let status_symbol = match check.status {
                CheckStatus::Pass => "✓".green(),
                CheckStatus::Warning => "!".yellow(),
                CheckStatus::Error => "✗".red(),
            };
            println!("  {} {} - {}", status_symbol, check.name.bold(), check.message);
        }
    }
    Ok(())
}
