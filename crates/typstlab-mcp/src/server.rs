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

    /// Read a line from stdin
    fn read_line() -> std::io::Result<String> {
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer)?;
        Ok(buffer)
    }

    /// Run the MCP server instdio mode
    pub fn run_stdio_server(root: PathBuf) -> anyhow::Result<()> {
        let server = Self::new(root)?;

        eprintln!("MCP Server running on stdio...");

        loop {
            let line = Self::read_line()?;
            if line.is_empty() {
                break; // EOF
            }

            let message: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Failed to parse JSON: {}", e);
                    continue;
                }
            };

            // Basic JSON-RPC 2.0 handling
            // Note: This is a simplified implementation. Real MCP requires fuller JSON-RPC support.
            // But for now we just wrap our specific tools.

            // For now, this is a placeholder.
            // The `rmcp` crate integration is planned but not fully wired here yet.
            // We'll implement a basic dispatcher for testing `typstlab mcp`.

            if let Some(method) = message.get("method").and_then(|m| m.as_str()) {
                if method == "tools/list" {
                    let tools = server.list_tools();
                    let response = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": message.get("id"),
                        "result": {
                            "tools": tools
                        }
                    });
                    println!("{}", serde_json::to_string(&response)?);
                } else if method == "resources/list" {
                    let resources = server.list_resources();
                    let response = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": message.get("id"),
                        "result": {
                            "resources": resources
                        }
                    });
                    println!("{}", serde_json::to_string(&response)?);
                } else if method == "tools/call" {
                    // Extract params
                    if let Some(params) = message.get("params") {
                        let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        let args = params
                            .get("arguments")
                            .cloned()
                            .unwrap_or(serde_json::json!({}));

                        match server.handle_tool_call(name, args) {
                            Ok(result) => {
                                let response = serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": message.get("id"),
                                    "result": result
                                });
                                println!("{}", serde_json::to_string(&response)?);
                            }
                            Err(e) => {
                                let response = serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": message.get("id"),
                                    "error": {
                                        "code": -32603,
                                        "message": e.to_string()
                                    }
                                });
                                println!("{}", serde_json::to_string(&response)?);
                            }
                        }
                    }
                } else if method == "resources/read"
                    && let Some(params) = message.get("params")
                {
                    let uri = params.get("uri").and_then(|u| u.as_str()).unwrap_or("");
                    match server.read_resource(uri) {
                        Ok(content) => {
                            let response = serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": message.get("id"),
                                "result": {
                                    "contents": [ content ]
                                }
                            });
                            println!("{}", serde_json::to_string(&response)?);
                        }
                        Err(e) => {
                            let response = serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": message.get("id"),
                                "error": {
                                    "code": -32603,
                                    "message": e.to_string()
                                }
                            });
                            println!("{}", serde_json::to_string(&response)?);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn list_resources(&self) -> Vec<Resource> {
        vec![
            Resource {
                uri: "typstlab://rules".to_string(),
                name: "Project Rules".to_string(),
                mime_type: Some("application/json".to_string()),
                description: Some("Project configuration from typstlab.toml".to_string()),
            },
            Resource {
                uri: "typstlab://docs".to_string(),
                name: "Documentation".to_string(),
                mime_type: Some("text/markdown".to_string()),
                description: Some("Generated documentation from .typstlab/kb".to_string()),
            },
        ]
    }

    /// Read a specific resource
    pub fn read_resource(&self, uri: &str) -> Result<ResourceContent> {
        if uri == "typstlab://rules" {
            let content = serde_json::to_string_pretty(self.project.config())?;
            return Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                text: content,
            });
        }

        if let Some(path_str) = uri.strip_prefix("typstlab://docs/") {
            // Defend against path traversal
            let path = std::path::Path::new(path_str);
            if path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                return Err(typstlab_core::error::TypstlabError::Generic(format!(
                    "Invalid path (traversal attempt): {}",
                    path_str
                )));
            }

            let docs_root = self.project.root.join(".typstlab/kb/docs");
            let file_path = docs_root.join(path);

            // Check if file is within docs root (canonicalization check)
            // Note: This requires file existence, so we read it directly.
            if !file_path.exists() {
                return Err(typstlab_core::error::TypstlabError::Generic(format!(
                    "File not found: {}",
                    path_str
                )));
            }

            let text = std::fs::read_to_string(file_path)?;
            return Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("text/markdown".to_string()),
                text,
            });
        }

        Err(typstlab_core::error::TypstlabError::Generic(format!(
            "Unknown resource: {}",
            uri
        )))
    }

    /// Handle a tool call
    pub fn handle_tool_call(&self, tool_name: &str, input: Value) -> Result<Value> {
        match tool_name {
            "rules_list" => {
                let input: tools::rules::RulesListInput = serde_json::from_value(input)?;
                let output = tools::rules::rules_list(input, &self.project.root)?;
                Ok(serde_json::to_value(output)?)
            }
            "rules_get" => {
                let input: tools::rules::RulesGetInput = serde_json::from_value(input)?;
                let output = tools::rules::rules_get(input, &self.project.root)?;
                Ok(serde_json::to_value(output)?)
            }
            "rules_page" => {
                let input: tools::rules::RulesPageInput = serde_json::from_value(input)?;
                let output = tools::rules::rules_page(input, &self.project.root)?;
                Ok(serde_json::to_value(output)?)
            }
            "rules_search" => {
                let input: tools::rules::RulesSearchInput = serde_json::from_value(input)?;
                let output = tools::rules::rules_search(input, &self.project.root)?;
                Ok(serde_json::to_value(output)?)
            }
            "cmd_generate" => {
                let args: GenerateArgs = serde_json::from_value(input)?;
                typstlab_core::project::generate_paper(&self.project, &args.paper_id)?;
                Ok(serde_json::json!({
                    "status": "success",
                    "message": format!("Successfully generated paper artifacts: {}", args.paper_id)
                }))
            }
            "docs_browse" => {
                let input: tools::docs::DocsBrowseInput = serde_json::from_value(input)?;
                let output = tools::docs::docs_browse(input, &self.project.root)
                    .map_err(|e| typstlab_core::error::TypstlabError::Generic(e.to_string()))?;
                Ok(serde_json::to_value(output)?)
            }
            "docs_search" => {
                let input: tools::docs::DocsSearchInput = serde_json::from_value(input)?;
                let output = tools::docs::docs_search(input, &self.project.root)
                    .map_err(|e| typstlab_core::error::TypstlabError::Generic(e.to_string()))?;
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
            ToolInfo {
                name: "cmd_generate".to_string(),
                description: "Generate paper templates (does not compile)".to_string(),
                safety: Safety {
                    network: true, // May download packages
                    reads: true,
                    writes: true, // Writes artifacts
                    writes_sot: true,
                },
            },
            ToolInfo {
                name: "docs_browse".to_string(),
                description: "Browse documentation directory structure".to_string(),
                safety: Safety {
                    network: false,
                    reads: true,
                    writes: false,
                    writes_sot: false,
                },
            },
            ToolInfo {
                name: "docs_search".to_string(),
                description: "Search within documentation files".to_string(),
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

#[derive(serde::Deserialize)]
struct GenerateArgs {
    paper_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub safety: Safety,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Safety {
    pub network: bool,
    pub reads: bool,
    pub writes: bool,
    pub writes_sot: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub mime_type: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
        assert_eq!(tools.len(), 7);
        assert!(tools.iter().any(|t| t.name == "rules_list"));
        assert!(tools.iter().any(|t| t.name == "rules_get"));
        assert!(tools.iter().any(|t| t.name == "rules_page"));
        assert!(tools.iter().any(|t| t.name == "rules_search"));
        assert!(tools.iter().any(|t| t.name == "cmd_generate"));
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

    #[test]
    fn test_read_docs_resource() {
        let temp = temp_dir_in_workspace();
        create_test_project_config(temp.path());

        // Create a dummy document file
        let docs_dir = temp.path().join(".typstlab/kb/docs");
        std::fs::create_dir_all(&docs_dir).unwrap();
        std::fs::write(docs_dir.join("intro.md"), "# Introduction").unwrap();

        let server = McpServer::new(temp.path().to_path_buf()).unwrap();

        // Test listing
        let resources = server.list_resources();
        assert!(resources.iter().any(|r| r.uri == "typstlab://docs"));

        // Test reading
        let content = server.read_resource("typstlab://docs/intro.md").unwrap();
        assert_eq!(content.text, "# Introduction");
    }

    #[test]
    fn test_tool_cmd_generate_success() {
        let temp = temp_dir_in_workspace();
        create_test_project_config(temp.path());

        // Create a dummy paper config
        let paper_dir = temp.path().join("papers/paper1");
        std::fs::create_dir_all(&paper_dir).unwrap();
        std::fs::write(
            paper_dir.join("paper.toml"),
            r#"
[paper]
id = "paper1"
title = "Test Paper"
language = "en"
date = "2026-01-14"

[output]
name = "paper1"
"#,
        )
        .unwrap();

        let server = McpServer::new(temp.path().to_path_buf()).unwrap();

        // Check tool listing
        let tools = server.list_tools();
        assert!(tools.iter().any(|t| t.name == "cmd_generate"));

        // Call cmd_generate tool
        let args = serde_json::json!({
            "paper_id": "paper1"
        });

        let result = server.handle_tool_call("cmd_generate", args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_docs_browse() -> anyhow::Result<()> {
        let temp = temp_dir_in_workspace();
        create_test_project_config(temp.path());

        // Setup docs directory structure (.typstlab/kb/docs based on Phase 2 implementation)
        let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
        std::fs::create_dir_all(&docs_dir)?;
        std::fs::write(docs_dir.join("intro.md"), "# Intro")?;
        std::fs::create_dir(docs_dir.join("reference"))?;
        std::fs::write(docs_dir.join("reference/syntax.md"), "# Syntax")?;

        let server = McpServer::new(temp.path().to_path_buf()).unwrap();

        // 1. Verify tool exists in list
        let tools = server.list_tools();
        assert!(tools.iter().any(|t| t.name == "docs_browse"));

        // 2. Browse root
        let args = serde_json::json!({});
        // Note: Default path="" implied
        let result = server.handle_tool_call("docs_browse", args)?;

        // Check for items array
        let items = result["items"]
            .as_array()
            .expect("result should contain items array");
        // Should contain intro.md (file) and reference (dir)
        assert!(
            items
                .iter()
                .any(|i| i["name"] == "intro.md" && i["type"] == "file")
        );
        assert!(
            items
                .iter()
                .any(|i| i["name"] == "reference" && i["type"] == "directory")
        );

        // 3. Browse subdirectory
        let args_sub = serde_json::json!({
            "path": "reference"
        });
        let result_sub = server.handle_tool_call("docs_browse", args_sub)?;
        let items_sub = result_sub["items"]
            .as_array()
            .expect("result should contain items array");
        assert!(
            items_sub
                .iter()
                .any(|i| i["name"] == "syntax.md" && i["type"] == "file")
        );

        Ok(())
    }

    #[test]
    fn test_tool_docs_search() -> anyhow::Result<()> {
        let temp = temp_dir_in_workspace();
        create_test_project_config(temp.path());

        let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
        std::fs::create_dir_all(&docs_dir)?;

        // Setup files for search
        std::fs::write(docs_dir.join("a.md"), "This is a search test.")?;
        std::fs::write(docs_dir.join("b.md"), "Another file with search keyword.")?;
        std::fs::write(docs_dir.join("c.md"), "No query match here.")?;

        let server = McpServer::new(temp.path().to_path_buf()).unwrap();

        // 1. Verify tool exists
        let tools = server.list_tools();
        assert!(tools.iter().any(|t| t.name == "docs_search"));

        // 2. Search for "search"
        let args = serde_json::json!({
            "query": "search"
        });
        let result = server.handle_tool_call("docs_search", args)?;

        let matches = result["matches"]
            .as_array()
            .expect("result should contain matches array");
        // Should find "search" in a.md and b.md
        assert_eq!(matches.len(), 2);

        // Check paths exist in matches
        let paths: Vec<String> = matches
            .iter()
            .map(|m| m["path"].as_str().unwrap().to_string())
            .collect();

        assert!(paths.contains(&"a.md".to_string()));
        assert!(paths.contains(&"b.md".to_string()));

        Ok(())
    }
}
