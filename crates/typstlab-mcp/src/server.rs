use crate::tools;
use serde_json::Value;
use std::path::PathBuf;
use typstlab_core::error::Result;

use anyhow::Context;
use typstlab_core::project::Project;

/// MCP Server for typstlab
pub struct McpServer {
    project: Project,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(root: PathBuf) -> anyhow::Result<Self> {
        let project = Project::find_root(&root)
            .context("Failed to search for project root")?
            .ok_or_else(|| anyhow::anyhow!("Project root not found in {}", root.display()))?;

        Ok(Self { project })
    }

    /// List available resources
    pub fn list_resources(&self) -> Vec<Resource> {
        vec![Resource {
            uri: "typstlab://rules".to_string(),
            name: "Project Rules".to_string(),
            mime_type: Some("application/json".to_string()),
            description: Some("Project configuration from typstlab.toml".to_string()),
        }]
    }

    /// Read a specific resource
    pub fn read_resource(&self, uri: &str) -> Result<ResourceContent> {
        match uri {
            "typstlab://rules" => {
                let content = serde_json::to_string_pretty(self.project.config())?;
                Ok(ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("application/json".to_string()),
                    text: content,
                })
            }
            _ => Err(typstlab_core::error::TypstlabError::Generic(format!(
                "Unknown resource: {}",
                uri
            ))),
        }
    }

    /// Handle a tool call
    pub fn handle_tool_call(&self, tool_name: &str, input: Value) -> Result<Value> {
        match tool_name {
            "rules_list" => {
                let input: tools::RulesListInput = serde_json::from_value(input)?;
                let output = tools::rules_list(input, &self.project.root)?;
                Ok(serde_json::to_value(output)?)
            }
            "rules_get" => {
                let input: tools::RulesGetInput = serde_json::from_value(input)?;
                let output = tools::rules_get(input, &self.project.root)?;
                Ok(serde_json::to_value(output)?)
            }
            "rules_page" => {
                let input: tools::RulesPageInput = serde_json::from_value(input)?;
                let output = tools::rules_page(input, &self.project.root)?;
                Ok(serde_json::to_value(output)?)
            }
            "rules_search" => {
                let input: tools::RulesSearchInput = serde_json::from_value(input)?;
                let output = tools::rules_search(input, &self.project.root)?;
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

#[derive(Debug, Clone)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub mime_type: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResourceContent {
    pub uri: String,
    pub mime_type: Option<String>,
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use typstlab_testkit::temp_dir_in_workspace;

    fn create_test_project_config(root: &std::path::Path) {
        let config = r#"
[project]
name = "test-project"
init_date = "2026-01-14"

[typst]
version = "0.12.0"
"#;
        std::fs::write(root.join("typstlab.toml"), config).unwrap();
    }

    #[test]
    fn test_list_tools() {
        let temp = temp_dir_in_workspace();
        create_test_project_config(temp.path());

        let server = McpServer::new(temp.path().to_path_buf()).unwrap();
        let tools = server.list_tools();
        assert_eq!(tools.len(), 4);
        assert!(tools.iter().any(|t| t.name == "rules_list"));
        assert!(tools.iter().any(|t| t.name == "rules_get"));
        assert!(tools.iter().any(|t| t.name == "rules_page"));
        assert!(tools.iter().any(|t| t.name == "rules_search"));
    }

    #[test]
    fn test_init_with_valid_project() {
        let temp = temp_dir_in_workspace();
        create_test_project_config(temp.path());

        let server = McpServer::new(temp.path().to_path_buf());
        assert!(server.is_ok());
    }

    #[test]
    fn test_init_fails_without_config() {
        let temp = temp_dir_in_workspace();
        // Do not create typstlab.toml

        let server = McpServer::new(temp.path().to_path_buf());
        assert!(server.is_err());
    }

    #[test]
    fn test_list_resources() {
        let temp = temp_dir_in_workspace();
        create_test_project_config(temp.path());

        let server = McpServer::new(temp.path().to_path_buf()).unwrap();
        let resources = server.list_resources();
        assert!(resources.iter().any(|r| r.uri == "typstlab://rules"));
    }

    #[test]
    fn test_read_rules_resource() {
        let temp = temp_dir_in_workspace();
        create_test_project_config(temp.path());

        let server = McpServer::new(temp.path().to_path_buf()).unwrap();
        let content = server.read_resource("typstlab://rules").unwrap();

        let json: serde_json::Value = serde_json::from_str(&content.text).unwrap();
        assert_eq!(json["project"]["name"], "test-project");
    }
}
