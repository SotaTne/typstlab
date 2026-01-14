//! Status command - show project health

use crate::context::Context;
use anyhow::Result;
use colored::Colorize;
use serde_json::json;
use typstlab_core::status::{engine::StatusEngine, schema::CheckStatus};

/// Show project status
///
/// # Arguments
///
/// * `paper` - Optional paper ID to filter status check
/// * `json` - Output as JSON if true
/// * `verbose` - Enable verbose output if true
///
/// # Exit Code
///
/// Always exits with code 0 (per DESIGN.md 5.1 Exit Code Policy).
/// Errors in checks are reported in the output, not via exit code.
pub fn run(paper: Option<String>, json: bool, verbose: bool) -> Result<()> {
    let ctx = Context::new(verbose)?;
    let engine = StatusEngine::new();

    let report = engine.run(&ctx.project, paper.as_deref());

    if json {
        render_json(&ctx, &report)?;
    } else {
        render_human(&ctx, &report, verbose);
    }

    // Always exit 0 (per DESIGN.md 5.1 Exit Code Policy)
    Ok(())
}

/// Render status report as JSON
fn render_json(ctx: &Context, report: &typstlab_core::status::schema::StatusReport) -> Result<()> {
    let timestamp = chrono::Utc::now().to_rfc3339();

    let output = json!({
        "schema_version": "1.0",
        "project": {
            "name": ctx.project.config().project.name,
            "root": ctx.project.root.display().to_string(),
        },
        "timestamp": timestamp,
        "overall_status": report.overall_status,
        "checks": report.checks,
        "actions": report.actions,
        "paper_filter": report.paper_filter,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Render status report in human-readable format
fn render_human(
    _ctx: &Context,
    report: &typstlab_core::status::schema::StatusReport,
    verbose: bool,
) {
    render_paper_filter(&report.paper_filter);
    render_checks(&report.checks, verbose);
    render_actions(&report.actions);
    render_summary(&report.overall_status, &report.checks);
}

/// Render paper filter if specified
fn render_paper_filter(paper_filter: &Option<String>) {
    if let Some(paper_id) = paper_filter {
        println!("{} Checking paper: {}\n", "→".cyan(), paper_id);
    }
}

/// Render check results with status icons
fn render_checks(checks: &[typstlab_core::status::schema::Check], verbose: bool) {
    for check in checks {
        let icon = status_icon(&check.status);
        let status_str = format!("[{}]", status_text(&check.status));

        println!("{} {} check {}", icon, check.name, status_str);

        // Print messages
        for msg in &check.messages {
            println!("  - {}", msg);
        }

        if verbose && check.messages.is_empty() {
            println!("  (no issues)");
        }
    }
}

/// Render suggested actions if any
fn render_actions(actions: &[typstlab_core::status::schema::SuggestedAction]) {
    if actions.is_empty() {
        return;
    }

    println!("\n{} Suggested actions:", "→".cyan());
    for action in actions {
        match action {
            typstlab_core::status::schema::SuggestedAction::RunCommand {
                command,
                description,
            } => {
                println!("  → {} ({})", command, description);
            }
            typstlab_core::status::schema::SuggestedAction::CreateFile { path, description } => {
                println!("  → Create {} ({})", path, description);
            }
            typstlab_core::status::schema::SuggestedAction::EditFile { path, description } => {
                println!("  → Edit {} ({})", path, description);
            }
            typstlab_core::status::schema::SuggestedAction::InstallTool { tool, url } => {
                println!("  → Install {} from {}", tool, url);
            }
        }
    }
}

/// Render overall summary
fn render_summary(overall_status: &CheckStatus, checks: &[typstlab_core::status::schema::Check]) {
    println!();
    let (error_count, warning_count) = count_issues(checks);
    match overall_status {
        CheckStatus::Pass => {
            println!("{} All checks passed", "✓".green().bold());
        }
        CheckStatus::Warning => {
            println!("{} {} warning(s)", "⚠".yellow().bold(), warning_count);
        }
        CheckStatus::Error => {
            println!(
                "{} {} error(s), {} warning(s)",
                "✗".red().bold(),
                error_count,
                warning_count
            );
        }
    }
}

/// Get status icon for check status
fn status_icon(status: &CheckStatus) -> String {
    match status {
        CheckStatus::Pass => "✓".green().to_string(),
        CheckStatus::Warning => "⚠".yellow().to_string(),
        CheckStatus::Error => "✗".red().to_string(),
    }
}

/// Get status text for check status
fn status_text(status: &CheckStatus) -> String {
    match status {
        CheckStatus::Pass => "PASS".green().to_string(),
        CheckStatus::Warning => "WARNING".yellow().to_string(),
        CheckStatus::Error => "ERROR".red().to_string(),
    }
}

/// Count errors and warnings in checks
fn count_issues(checks: &[typstlab_core::status::schema::Check]) -> (usize, usize) {
    let error_count = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Error)
        .count();
    let warning_count = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Warning)
        .count();
    (error_count, warning_count)
}
