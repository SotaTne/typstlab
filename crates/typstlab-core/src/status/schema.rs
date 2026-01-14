//! Status report schema definitions

use serde::{Deserialize, Serialize};

/// Status of a check
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warning,
    Error,
}

/// Suggested action to resolve an issue
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SuggestedAction {
    RunCommand {
        command: String,
        description: String,
    },
    CreateFile {
        path: String,
        description: String,
    },
    EditFile {
        path: String,
        description: String,
    },
    InstallTool {
        tool: String,
        url: String,
    },
}

/// Individual check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub name: String,
    pub status: CheckStatus,
    pub messages: Vec<String>,
}

/// Status report structure
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusReport {
    pub overall_status: CheckStatus,
    pub checks: Vec<Check>,
    pub actions: Vec<SuggestedAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper_filter: Option<String>,
}

impl StatusReport {
    pub fn empty() -> Self {
        Self {
            overall_status: CheckStatus::Pass,
            checks: vec![],
            actions: vec![],
            paper_filter: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_status_equality() {
        assert_eq!(CheckStatus::Pass, CheckStatus::Pass);
        assert_eq!(CheckStatus::Warning, CheckStatus::Warning);
        assert_eq!(CheckStatus::Error, CheckStatus::Error);
        assert_ne!(CheckStatus::Pass, CheckStatus::Warning);
    }

    #[test]
    fn test_check_status_serialization() {
        let pass = serde_json::to_string(&CheckStatus::Pass).unwrap();
        assert_eq!(pass, r#""pass""#);

        let warning = serde_json::to_string(&CheckStatus::Warning).unwrap();
        assert_eq!(warning, r#""warning""#);

        let error = serde_json::to_string(&CheckStatus::Error).unwrap();
        assert_eq!(error, r#""error""#);
    }

    #[test]
    fn test_suggested_action_serialization() {
        let action = SuggestedAction::RunCommand {
            command: "typst --version".to_string(),
            description: "Check Typst version".to_string(),
        };

        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json["type"], "run_command");
        assert_eq!(json["command"], "typst --version");
    }

    #[test]
    fn test_check_construction() {
        let check = Check {
            name: "environment".to_string(),
            status: CheckStatus::Pass,
            messages: vec!["All directories present".to_string()],
        };

        assert_eq!(check.name, "environment");
        assert_eq!(check.status, CheckStatus::Pass);
        assert_eq!(check.messages.len(), 1);
    }

    #[test]
    fn test_status_report_empty() {
        let report = StatusReport::empty();
        assert_eq!(report.overall_status, CheckStatus::Pass);
        assert_eq!(report.checks.len(), 0);
        assert_eq!(report.actions.len(), 0);
        assert_eq!(report.paper_filter, None);
    }

    #[test]
    fn test_status_report_with_checks() {
        let report = StatusReport {
            overall_status: CheckStatus::Warning,
            checks: vec![
                Check {
                    name: "environment".to_string(),
                    status: CheckStatus::Pass,
                    messages: vec![],
                },
                Check {
                    name: "typst".to_string(),
                    status: CheckStatus::Warning,
                    messages: vec!["Version mismatch".to_string()],
                },
            ],
            actions: vec![SuggestedAction::InstallTool {
                tool: "typst".to_string(),
                url: "https://github.com/typst/typst".to_string(),
            }],
            paper_filter: Some("paper1".to_string()),
        };

        assert_eq!(report.overall_status, CheckStatus::Warning);
        assert_eq!(report.checks.len(), 2);
        assert_eq!(report.actions.len(), 1);
        assert_eq!(report.paper_filter, Some("paper1".to_string()));
    }

    #[test]
    fn test_status_report_serialization() {
        let report = StatusReport {
            overall_status: CheckStatus::Error,
            checks: vec![Check {
                name: "build".to_string(),
                status: CheckStatus::Error,
                messages: vec!["main.typ not found".to_string()],
            }],
            actions: vec![SuggestedAction::CreateFile {
                path: "papers/paper1/main.typ".to_string(),
                description: "Create main entry file".to_string(),
            }],
            paper_filter: None,
        };

        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["overall_status"], "error");
        assert_eq!(json["checks"][0]["name"], "build");
        assert_eq!(json["actions"][0]["type"], "create_file");
    }
}
