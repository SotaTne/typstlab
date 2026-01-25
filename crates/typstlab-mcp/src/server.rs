use crate::context::McpContext;
use crate::handlers::cmd::CmdTool;
use crate::handlers::docs::DocsTool;
use crate::handlers::rules::RulesTool;
use rmcp::{
    RoleServer, ServerHandler,
    handler::server::router::{prompt::PromptRouter, tool::ToolRouter},
    model::*,
    service::RequestContext,
};
use std::path::Path;

#[cfg(test)]
use typstlab_testkit::temp_dir_in_workspace;

const MAX_RESOURCE_BYTES: u64 = 1024 * 1024;

pub struct TypstlabServer {
    pub context: McpContext,
    pub tool_router: ToolRouter<TypstlabServer>,
    pub prompt_router: PromptRouter<TypstlabServer>,
}

impl TypstlabServer {
    pub fn new(context: McpContext, offline: bool) -> Self {
        let mut tool_router = ToolRouter::new();
        tool_router.merge(DocsTool.into_router());

        // Use offline-safe router when offline mode is enabled
        // This allows read-only tools (cmd_status, cmd_typst_docs_status) to work offline
        // while excluding network-dependent tools (cmd_generate, cmd_build)
        if offline {
            tool_router.merge(CmdTool.into_router_offline());
        } else {
            tool_router.merge(CmdTool.into_router());
        }

        tool_router.merge(RulesTool.into_router());

        Self {
            context,
            tool_router,
            prompt_router: PromptRouter::new(),
        }
    }

    pub async fn run_stdio_server(root: std::path::PathBuf, offline: bool) -> anyhow::Result<()> {
        let project = typstlab_core::project::Project::find_root(&root)?.ok_or_else(|| {
            anyhow::anyhow!(
                "Project not found: typstlab.toml not found in current or parent directories"
            )
        })?;
        let context = McpContext::new(project.root);
        let server = Self::new(context, offline);
        let transport = rmcp::transport::io::stdio();
        let service = rmcp::serve_server(server, transport).await?;
        service.waiting().await?;
        Ok(())
    }
}

pub type McpServer = TypstlabServer;

impl ServerHandler for TypstlabServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                resources: Some(ResourcesCapability {
                    subscribe: Some(false),
                    list_changed: Some(false),
                }),
                prompts: Some(PromptsCapability {
                    list_changed: Some(false),
                }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "typstlab".into(),
                version: "0.1.0".into(),
                icons: None,
                title: Some("typstlab".into()),
                website_url: None,
            },
            instructions: Some(
                "typstlab MCP server for managing Typst projects and documentation.".to_string(),
            ),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(self.tool_router.list_all()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        self.tool_router
            .call(rmcp::handler::server::tool::ToolCallContext::new(
                self, request, context,
            ))
            .await
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult::with_all_items(vec![
            Resource::new(
                RawResource {
                    uri: "typstlab://rules".into(),
                    name: "rules".into(),
                    title: None,
                    description: Some("Project rules and guidelines".into()),
                    mime_type: Some("text/markdown".into()),
                    size: None,
                    icons: None,
                    meta: None,
                },
                None,
            ),
            Resource::new(
                RawResource {
                    uri: "typstlab://docs".into(),
                    name: "docs".into(),
                    title: None,
                    description: Some("Typst documentation".into()),
                    mime_type: Some("text/markdown".into()),
                    size: None,
                    icons: None,
                    meta: None,
                },
                None,
            ),
        ]))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        self.read_resource_by_uri(request.uri.as_str()).await
    }
}

impl TypstlabServer {
    async fn read_resource_by_uri(&self, uri: &str) -> Result<ReadResourceResult, ErrorData> {
        if uri == "typstlab://rules" {
            let root = self.context.project_root.join("rules");
            if !root.exists() {
                return Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(
                        serde_json::to_string(&serde_json::json!({ "items": [] }))
                            .map_err(|err| ErrorData::internal_error(err.to_string(), None))?,
                        uri,
                    )],
                });
            }
            let items =
                crate::handlers::rules::rules_browse_items(&root, &self.context.project_root)
                    .await?;
            let text = serde_json::to_string(&serde_json::json!({
                "items": items,
                "missing": false,
            }))
            .map_err(|err| ErrorData::internal_error(err.to_string(), None))?;
            return Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(text, uri)],
            });
        }

        if uri == "typstlab://docs" {
            let root = self.context.project_root.join(".typstlab/kb/typst/docs");
            if !root.exists() {
                return Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(
                        serde_json::to_string(&serde_json::json!({
                            "items": [],
                            "missing": true,
                        }))
                        .map_err(|err| ErrorData::internal_error(err.to_string(), None))?,
                        uri,
                    )],
                });
            }
            let mut items = Vec::new();
            let mut dir = tokio::fs::read_dir(&root)
                .await
                .map_err(|err| ErrorData::internal_error(err.to_string(), None))?;
            while let Some(entry) = dir
                .next_entry()
                .await
                .map_err(|err| ErrorData::internal_error(err.to_string(), None))?
            {
                let file_type = entry
                    .file_type()
                    .await
                    .map_err(|err| ErrorData::internal_error(err.to_string(), None))?;
                if file_type.is_symlink() {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let entry_path = entry.path();
                let entry_type = if entry_path.is_dir() {
                    "directory"
                } else if entry_path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                    "file"
                } else {
                    continue;
                };
                items.push(serde_json::json!({
                    "name": name,
                    "type": entry_type,
                }));
            }
            items.sort_by_key(|i| {
                i.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string()
            });
            let text = serde_json::to_string(&serde_json::json!({ "items": items }))
                .map_err(|err| ErrorData::internal_error(err.to_string(), None))?;
            return Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(text, uri)],
            });
        }

        if let Some(path) = uri.strip_prefix("typstlab://rules/") {
            if path.is_empty() {
                return Err(ErrorData::invalid_params(
                    "Resource path required".to_string(),
                    None,
                ));
            }
            let _requested = std::path::Path::new(path);
            // resolve_rules_path expects paths starting with "rules/" or "papers/<id>/rules"
            // URI: typstlab://rules/guidelines.md -> path: "guidelines.md" -> need: "rules/guidelines.md"
            let full_path = format!("rules/{}", path);
            let target = crate::handlers::rules::resolve_rules_path(
                &self.context.project_root,
                Path::new(&full_path),
            )
            .await?;
            if !target.exists() || !target.is_file() {
                return Err(ErrorData::resource_not_found(
                    format!("Resource not found: {}", uri),
                    None,
                ));
            }
            if target.extension().and_then(|ext| ext.to_str()) != Some("md") {
                return Err(ErrorData::resource_not_found(
                    format!("Resource not found: {}", uri),
                    None,
                ));
            }
            let metadata = tokio::fs::metadata(&target)
                .await
                .map_err(|err| ErrorData::internal_error(err.to_string(), None))?;
            if metadata.len() > MAX_RESOURCE_BYTES {
                return Err(crate::errors::error_with_code(
                    crate::errors::FILE_TOO_LARGE,
                    format!("Resource exceeds {} bytes", MAX_RESOURCE_BYTES),
                ));
            }
            let content = tokio::fs::read_to_string(&target)
                .await
                .map_err(|err| ErrorData::internal_error(err.to_string(), None))?;
            return Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(content, uri)],
            });
        }

        if let Some(path) = uri.strip_prefix("typstlab://docs/") {
            if path.is_empty() {
                return Err(ErrorData::invalid_params(
                    "Resource path required".to_string(),
                    None,
                ));
            }
            let docs_root = self.context.project_root.join(".typstlab/kb/typst/docs");
            let requested = std::path::Path::new(path);
            let target = crate::handlers::docs::resolve_docs_path(&docs_root, requested).await?;
            if !target.exists() || !target.is_file() {
                return Err(ErrorData::resource_not_found(
                    format!("Resource not found: {}", uri),
                    None,
                ));
            }
            if target.extension().and_then(|ext| ext.to_str()) != Some("md") {
                return Err(ErrorData::resource_not_found(
                    format!("Resource not found: {}", uri),
                    None,
                ));
            }
            let metadata = tokio::fs::metadata(&target)
                .await
                .map_err(|err| ErrorData::internal_error(err.to_string(), None))?;
            if metadata.len() > MAX_RESOURCE_BYTES {
                return Err(crate::errors::error_with_code(
                    crate::errors::FILE_TOO_LARGE,
                    format!("Resource exceeds {} bytes", MAX_RESOURCE_BYTES),
                ));
            }
            let content = tokio::fs::read_to_string(&target)
                .await
                .map_err(|err| ErrorData::internal_error(err.to_string(), None))?;
            return Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(content, uri)],
            });
        }

        Err(ErrorData::resource_not_found(
            format!("Resource not found: {}", uri),
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tokio::fs;
    use typstlab_testkit::temp_dir_in_workspace;

    #[tokio::test]
    async fn test_server_info() {
        let ctx = McpContext::new(PathBuf::from("."));
        let server = TypstlabServer::new(ctx, false);
        let info = server.get_info();
        assert_eq!(info.server_info.name, "typstlab");
        assert_eq!(info.protocol_version, ProtocolVersion::V_2024_11_05);
    }

    #[tokio::test]
    async fn test_list_tools() {
        let ctx = McpContext::new(PathBuf::from("."));
        let server = TypstlabServer::new(ctx, false);

        let tools = server.tool_router.list_all();
        let names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
        assert!(names.contains(&"rules_browse".to_string()));
        assert!(names.contains(&"cmd_build".to_string()));
    }

    #[tokio::test]
    async fn test_list_resources() {
        let ctx = McpContext::new(PathBuf::from("."));
        let server = TypstlabServer::new(ctx, false);

        // We can't easily call list_resources without RequestContext,
        // but we can test it handles the URIs we expect in read_resource later.
        // For now, let's just ensure the server initializes.
        assert!(!server.tool_router.list_all().is_empty());
    }

    #[tokio::test]
    async fn test_read_resource_rules_root_returns_listing() {
        let temp = temp_dir_in_workspace();
        let rules_dir = temp.path().join("rules");
        fs::create_dir_all(&rules_dir).await.unwrap();
        fs::write(rules_dir.join("a.md"), "# A").await.unwrap();

        let ctx = McpContext::new(temp.path().to_path_buf());
        let server = TypstlabServer::new(ctx, false);

        let res = server
            .read_resource_by_uri("typstlab://rules")
            .await
            .expect("read resource");
        let content = &res.contents[0];
        let text = match content {
            ResourceContents::TextResourceContents { text, .. } => text,
            _ => panic!("expected text content"),
        };
        assert!(text.contains("\"path\":\"rules/a.md\""));
    }

    #[tokio::test]
    async fn test_read_resource_docs_root_returns_listing() {
        let temp = temp_dir_in_workspace();
        let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
        fs::create_dir_all(&docs_dir).await.unwrap();
        fs::write(docs_dir.join("b.md"), "# B").await.unwrap();

        let ctx = McpContext::new(temp.path().to_path_buf());
        let server = TypstlabServer::new(ctx, false);

        let res = server
            .read_resource_by_uri("typstlab://docs")
            .await
            .expect("read resource");
        let content = &res.contents[0];
        let text = match content {
            ResourceContents::TextResourceContents { text, .. } => text,
            _ => panic!("expected text content"),
        };
        assert!(text.contains("\"name\":\"b.md\""));
    }
}
#[tokio::test]
async fn test_offline_mode_includes_status() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, true); // offline = true

    // cmd_status should be available in offline mode
    let tools = server.tool_router.list_all();
    let status_tool = tools.iter().find(|t| t.name == "cmd_status");
    assert!(
        status_tool.is_some(),
        "cmd_status should be available in offline mode"
    );
}

#[tokio::test]
async fn test_offline_mode_excludes_build() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, true); // offline = true

    // cmd_build should NOT be available in offline mode
    let tools = server.tool_router.list_all();
    let build_tool = tools.iter().find(|t| t.name == "cmd_build");
    assert!(
        build_tool.is_none(),
        "cmd_build should NOT be available in offline mode"
    );
}

#[tokio::test]
async fn test_offline_mode_excludes_generate() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, true); // offline = true

    // cmd_generate should NOT be available in offline mode
    let tools = server.tool_router.list_all();
    let generate_tool = tools.iter().find(|t| t.name == "cmd_generate");
    assert!(
        generate_tool.is_none(),
        "cmd_generate should NOT be available in offline mode"
    );
}
