use crate::tools;
use serde_json::Value;
use std::path::PathBuf;
use typstlab_core::error::Result;

/// MCP Server for typstlab
pub struct McpServer {
    project_root: PathBuf,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Handle a tool call
    pub fn handle_tool_call(&self, tool_name: &str, input: Value) -> Result<Value> {
        match tool_name {
            "rules_list" => {
                let input: tools::RulesListInput = serde_json::from_value(input)?;
                let output = tools::rules_list(input, &self.project_root)?;
                Ok(serde_json::to_value(output)?)
            }
            "rules_get" => {
                let input: tools::RulesGetInput = serde_json::from_value(input)?;
                let output = tools::rules_get(input, &self.project_root)?;
                Ok(serde_json::to_value(output)?)
            }
            "rules_page" => {
                let input: tools::RulesPageInput = serde_json::from_value(input)?;
                let output = tools::rules_page(input, &self.project_root)?;
                Ok(serde_json::to_value(output)?)
            }
            "rules_search" => {
                let input: tools::RulesSearchInput = serde_json::from_value(input)?;
                let output = tools::rules_search(input, &self.project_root)?;
                Ok(serde_json::to_value(output)?)
            }
            _ => Err(typstlab_core::error::TypstlabError::Generic(format!(
                "Unknown tool: {}",
                tool_name
            ))),
        }
    }

    /// List available tools
    pub fn list_tools(&self) -> Vec<ToolInfo> {
        vec![
            ToolInfo {
                name: "rules_list".to_string(),
                description: "List files in rules/ directories with pagination".to_string(),
                safety: Safety {
                    network: false,
                    reads: true,
                    writes: false,
                    writes_sot: false,
                },
            },
            ToolInfo {
                name: "rules_get".to_string(),
                description: "Retrieve full content of a rules file".to_string(),
                safety: Safety {
                    network: false,
                    reads: true,
                    writes: false,
                    writes_sot: false,
                },
            },
            ToolInfo {
                name: "rules_page".to_string(),
                description: "Retrieve file content in line-based chunks".to_string(),
                safety: Safety {
                    network: false,
                    reads: true,
                    writes: false,
                    writes_sot: false,
                },
            },
            ToolInfo {
                name: "rules_search".to_string(),
                description: "Full-text search across all rules files".to_string(),
                safety: Safety {
                    network: false,
                    reads: true,
                    writes: false,
                    writes_sot: false,
                },
            },
        ]
    }
}

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub safety: Safety,
}

#[derive(Debug, Clone)]
pub struct Safety {
    pub network: bool,
    pub reads: bool,
    pub writes: bool,
    pub writes_sot: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_tools() {
        let server = McpServer::new(PathBuf::from("/tmp/test"));
        let tools = server.list_tools();
        assert_eq!(tools.len(), 4);
        assert!(tools.iter().any(|t| t.name == "rules_list"));
        assert!(tools.iter().any(|t| t.name == "rules_get"));
        assert!(tools.iter().any(|t| t.name == "rules_page"));
        assert!(tools.iter().any(|t| t.name == "rules_search"));
    }
}
