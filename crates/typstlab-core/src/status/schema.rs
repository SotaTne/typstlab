//! Status report schema definitions

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Status of a check
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warning,
    Error,
}

/// Safety definition for actions
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Safety {
    pub network: bool,
    pub writes: bool,
    pub writes_sot: bool,
    pub reads: bool,
}

impl Safety {
    pub fn safe() -> Self {
        Self {
            network: false,
            writes: false,
            writes_sot: false,
            reads: false,
        }
    }
}

/// Action to resolve an issue
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Action {
    pub id: String,
    pub command: String,
    pub description: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,
    pub safety: Safety,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prerequisite: Option<Vec<String>>,
}

/// Individual check result
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Check {
    pub id: String,
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Status report structure
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StatusReport {
    pub overall_status: CheckStatus,
    pub checks: Vec<Check>,
    pub actions: Vec<Action>,
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
}
