//! Status report schema definitions

use serde::{Deserialize, Serialize};

/// Status report structure
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusReport {
    pub schema_version: String,
    pub project: Option<String>,
    pub time: Option<String>,
    pub summary: Option<String>,
    pub checks: Vec<String>,
    pub actions: Vec<String>,
}

impl StatusReport {
    pub fn empty() -> Self {
        Self {
            schema_version: "1.0".to_string(),
            project: None,
            time: None,
            summary: None,
            checks: vec![],
            actions: vec![],
        }
    }
}
