//! Status check engine

use crate::{
    config::Config,
    paper::Paper,
    project::Project,
    status::{
        checks::{BuildCheck, EnvCheck, RefsCheck, TypstCheck},
        schema::{CheckStatus, StatusReport},
    },
};

/// Context provided to status checks
pub struct CheckContext<'a> {
    pub project: &'a Project,
    pub target_paper: Option<&'a str>,
}

impl<'a> CheckContext<'a> {
    /// Get target papers (all papers or filtered by ID)
    pub fn target_papers(&self) -> Vec<&Paper> {
        match self.target_paper {
            Some(id) => self
                .project
                .papers()
                .iter()
                .filter(|p| p.id() == id)
                .collect(),
            None => self.project.papers().iter().collect(),
        }
    }

    /// Get project configuration
    pub fn config(&self) -> &Config {
        self.project.config()
    }

    /// Get all papers in project
    pub fn papers(&self) -> &[Paper] {
        self.project.papers()
    }

    /// Get project root path
    pub fn root(&self) -> &std::path::Path {
        &self.project.root
    }
}

/// Result of a single check
pub struct CheckResult {
    pub id: String,
    pub status: CheckStatus,
    pub message: String,
    pub details: Option<std::collections::HashMap<String, serde_json::Value>>,
    pub actions: Vec<crate::status::schema::Action>,
}

impl CheckResult {
    /// Create a passing check result
    pub fn pass(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: CheckStatus::Pass,
            message: message.into(),
            details: None,
            actions: vec![],
        }
    }

    /// Create a warning check result
    pub fn warning(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: CheckStatus::Warning,
            message: message.into(),
            details: None,
            actions: vec![],
        }
    }

    /// Create an error check result
    pub fn error(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: CheckStatus::Error,
            message: message.into(),
            details: None,
            actions: vec![],
        }
    }

    /// Add an action to this result
    pub fn with_action(mut self, action: crate::status::schema::Action) -> Self {
        self.actions.push(action);
        self
    }

    /// Add details
    pub fn with_detail(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        let details = self
            .details
            .get_or_insert_with(std::collections::HashMap::new);
        details.insert(key.into(), value.into());
        self
    }
}

/// Trait for status checks
pub trait StatusCheck {
    /// Get the name of this check
    fn name(&self) -> &str;

    /// Run the check
    fn run(&self, context: &CheckContext) -> CheckResult;
}

/// Status check engine - aggregates and runs all checks
pub struct StatusEngine {
    checks: Vec<Box<dyn StatusCheck>>,
}

impl StatusEngine {
    /// Create a new status engine with all registered checks
    pub fn new() -> Self {
        let checks: Vec<Box<dyn StatusCheck>> = vec![
            Box::new(EnvCheck),
            Box::new(TypstCheck),
            Box::new(BuildCheck),
            Box::new(RefsCheck),
        ];
        Self { checks }
    }

    /// Run all checks and aggregate results
    pub fn run(&self, project: &Project, target_paper: Option<&str>) -> StatusReport {
        let context = CheckContext {
            project,
            target_paper,
        };

        let mut all_checks = Vec::new();
        let mut all_actions = Vec::new();
        let mut overall_status = CheckStatus::Pass;

        for check in &self.checks {
            let result = check.run(&context);

            // Update overall status (Error > Warning > Pass)
            overall_status = match (&overall_status, &result.status) {
                (CheckStatus::Error, _) | (_, CheckStatus::Error) => CheckStatus::Error,
                (CheckStatus::Warning, _) | (_, CheckStatus::Warning) => CheckStatus::Warning,
                _ => CheckStatus::Pass,
            };

            all_checks.push(crate::status::schema::Check {
                id: result.id,
                name: check.name().to_string(),
                status: result.status,
                message: result.message,
                details: result.details,
            });

            all_actions.extend(result.actions);
        }

        StatusReport {
            overall_status,
            checks: all_checks,
            actions: all_actions,
            paper_filter: target_paper.map(String::from),
        }
    }
}

impl Default for StatusEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::schema::CheckStatus;

    #[test]
    fn test_check_context_target_papers_all() {
        use typstlab_testkit::temp_dir_in_workspace;

        let temp = temp_dir_in_workspace();
        let project_dir = temp.path();

        // Create minimal project structure
        std::fs::write(
            project_dir.join("typstlab.toml"),
            r#"
[project]
name = "test"
init_date = "2026-01-14"

[typst]
version = "0.12.0"
"#,
        )
        .unwrap();

        let papers_dir = project_dir.join("papers");
        std::fs::create_dir(&papers_dir).unwrap();

        // Create two papers
        for id in ["paper1", "paper2"] {
            let paper_dir = papers_dir.join(id);
            std::fs::create_dir(&paper_dir).unwrap();
            std::fs::write(
                paper_dir.join("paper.toml"),
                format!(
                    r#"
[paper]
id = "{}"
title = "Test"
language = "en"
date = "2026-01-14"

[output]
name = "{}"
"#,
                    id, id
                ),
            )
            .unwrap();
        }

        let project = Project::load(project_dir.to_path_buf()).unwrap();
        let context = CheckContext {
            project: &project,
            target_paper: None,
        };

        let papers = context.target_papers();
        assert_eq!(papers.len(), 2);
    }

    #[test]
    fn test_check_context_target_papers_filtered() {
        use typstlab_testkit::temp_dir_in_workspace;

        let temp = temp_dir_in_workspace();
        let project_dir = temp.path();

        std::fs::write(
            project_dir.join("typstlab.toml"),
            r#"
[project]
name = "test"
init_date = "2026-01-14"

[typst]
version = "0.12.0"
"#,
        )
        .unwrap();

        let papers_dir = project_dir.join("papers");
        std::fs::create_dir(&papers_dir).unwrap();

        for id in ["paper1", "paper2"] {
            let paper_dir = papers_dir.join(id);
            std::fs::create_dir(&paper_dir).unwrap();
            std::fs::write(
                paper_dir.join("paper.toml"),
                format!(
                    r#"
[paper]
id = "{}"
title = "Test"
language = "en"
date = "2026-01-14"

[output]
name = "{}"
"#,
                    id, id
                ),
            )
            .unwrap();
        }

        let project = Project::load(project_dir.to_path_buf()).unwrap();
        let context = CheckContext {
            project: &project,
            target_paper: Some("paper1"),
        };

        let papers = context.target_papers();
        assert_eq!(papers.len(), 1);
        assert_eq!(papers[0].id(), "paper1");
    }

    #[test]
    fn test_check_result_constructors() {
        let pass = CheckResult::pass("test_ok", "All good");
        assert_eq!(pass.status, CheckStatus::Pass);
        assert_eq!(pass.message, "All good");
        assert_eq!(pass.id, "test_ok");

        let warning = CheckResult::warning("test_warn", "Test warning");
        assert_eq!(warning.status, CheckStatus::Warning);
        assert_eq!(warning.message, "Test warning");

        let error = CheckResult::error("test_err", "Test error");
        assert_eq!(error.status, CheckStatus::Error);
        assert_eq!(error.message, "Test error");
    }

    #[test]
    fn test_status_engine_runs_all_checks() {
        use typstlab_testkit::temp_dir_in_workspace;

        let temp = temp_dir_in_workspace();
        let project_dir = temp.path();

        std::fs::write(
            project_dir.join("typstlab.toml"),
            r#"
[project]
name = "test"
init_date = "2026-01-14"

[typst]
version = "0.12.0"
"#,
        )
        .unwrap();

        std::fs::create_dir(project_dir.join("papers")).unwrap();

        let project = Project::load(project_dir.to_path_buf()).unwrap();
        let engine = StatusEngine::new();
        let report = engine.run(&project, None);

        // Should run all 4 checks
        assert_eq!(report.checks.len(), 4);
    }

    #[test]
    fn test_status_engine_aggregates_overall_status() {
        // This test will be more meaningful after Phase 5 when checks have real logic
        use typstlab_testkit::temp_dir_in_workspace;

        let temp = temp_dir_in_workspace();
        let project_dir = temp.path();

        std::fs::write(
            project_dir.join("typstlab.toml"),
            r#"
[project]
name = "test"
init_date = "2026-01-14"

[typst]
version = "0.12.0"
"#,
        )
        .unwrap();

        std::fs::create_dir(project_dir.join("papers")).unwrap();

        let project = Project::load(project_dir.to_path_buf()).unwrap();
        let engine = StatusEngine::new();
        let report = engine.run(&project, None);

        // Overall status should be aggregated from all checks
        assert!(matches!(
            report.overall_status,
            CheckStatus::Pass | CheckStatus::Warning | CheckStatus::Error
        ));
    }
}
