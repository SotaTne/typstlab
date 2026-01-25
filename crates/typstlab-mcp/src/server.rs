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

use typstlab_core::config::consts::search::MAX_FILE_BYTES;

#[derive(Clone)]
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
                version: env!("CARGO_PKG_VERSION").into(),
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
                    mime_type: Some("application/json".into()),
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
                    mime_type: Some("application/json".into()),
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
        context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        // Use cancellation token from request context
        self.read_resource_by_uri(request.uri.as_str(), context.ct)
            .await
    }
}

impl TypstlabServer {
    pub async fn test_read_resource_by_uri(
        &self,
        uri: &str,
    ) -> Result<ReadResourceResult, ErrorData> {
        // For testing, create a dummy token
        self.read_resource_by_uri(uri, tokio_util::sync::CancellationToken::new())
            .await
    }

    async fn read_resource_by_uri(
        &self,
        uri: &str,
        token: tokio_util::sync::CancellationToken,
    ) -> Result<ReadResourceResult, ErrorData> {
        if uri == "typstlab://rules" {
            return self.handle_rules_resource(uri, token).await;
        }
        if uri == "typstlab://docs" {
            return self.handle_docs_resource(uri, token).await;
        }

        if let Some(path) = uri.strip_prefix("typstlab://rules/") {
            return self.handle_rules_file(uri, path).await;
        }
        if let Some(path) = uri.strip_prefix("typstlab://docs/") {
            return self.handle_docs_file(uri, path).await;
        }

        Err(crate::errors::resource_not_found(format!(
            "Resource not found: {}",
            uri
        )))
    }

    async fn handle_rules_resource(
        &self,
        uri: &str,
        token: tokio_util::sync::CancellationToken,
    ) -> Result<ReadResourceResult, ErrorData> {
        let root = self.context.project_root.join("rules");
        let project_root = self.context.project_root.clone();
        let uri = uri.to_string();

        let browse_res = tokio::task::spawn_blocking(move || {
            crate::handlers::common::ops::browse_dir_sync(
                &root,
                &project_root,
                None, // relative_path
                &["md".to_string()],
                1000,
                token,
            )
        })
        .await
        .map_err(crate::errors::from_display)??;

        let text = serde_json::to_string(&serde_json::json!({
            "items": browse_res.items,
            "missing": browse_res.missing,
            "truncated": browse_res.truncated,
        }))
        .map_err(crate::errors::from_display)?;

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(text, uri)],
        })
    }

    async fn handle_docs_resource(
        &self,
        uri: &str,
        token: tokio_util::sync::CancellationToken,
    ) -> Result<ReadResourceResult, ErrorData> {
        let root = self.context.project_root.join(".typstlab/kb/typst/docs");
        let project_root = self.context.project_root.clone();
        let uri = uri.to_string();

        let browse_res = tokio::task::spawn_blocking(move || {
            // browse_dir_sync applies check_entry_safety internally to target_dir if it exists.
            // However, we want to ensure root safety even if it doesn't strictly exist yet?
            // browse_dir_sync returns missing=true if not exist.
            // But check_entry_safety is called AFTER exists() check in browse_dir_sync.
            // If it's a symlink to /etc, and exists, browse_dir_sync will fail with PATH_ESCAPE.
            crate::handlers::common::ops::browse_dir_sync(
                &root,
                &project_root,
                None,
                &["md".to_string()],
                1000,
                token,
            )
        })
        .await
        .map_err(crate::errors::from_display)??;

        let text = serde_json::to_string(&serde_json::json!({
            "items": browse_res.items,
            "missing": browse_res.missing,
            "truncated": browse_res.truncated,
        }))
        .map_err(crate::errors::from_display)?;

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(text, uri)],
        })
    }

    // Extracted file handlers to keep methods small
    async fn handle_rules_file(
        &self,
        uri: &str,
        path: &str,
    ) -> Result<ReadResourceResult, ErrorData> {
        if path.is_empty() {
            return Err(crate::errors::invalid_params("Resource path required"));
        }
        let rules_path = Path::new("rules").join(path);
        let target =
            crate::handlers::rules::resolve_rules_path(&self.context.project_root, &rules_path)
                .await?;

        self.read_file_resource(uri, &target).await
    }

    async fn handle_docs_file(
        &self,
        uri: &str,
        path: &str,
    ) -> Result<ReadResourceResult, ErrorData> {
        if path.is_empty() {
            return Err(crate::errors::invalid_params("Resource path required"));
        }
        let docs_root = self.context.project_root.join(".typstlab/kb/typst/docs");
        let target = crate::handlers::docs::resolve_docs_path(
            &self.context.project_root,
            &docs_root,
            Path::new(path),
        )
        .await?;

        self.read_file_resource(uri, &target).await
    }

    async fn read_file_resource(
        &self,
        uri: &str,
        target: &Path,
    ) -> Result<ReadResourceResult, ErrorData> {
        if !target.exists() || !target.is_file() {
            return Err(crate::errors::resource_not_found(format!(
                "Resource not found: {}",
                uri
            )));
        }
        if target.extension().and_then(|ext| ext.to_str()) != Some("md") {
            return Err(crate::errors::resource_not_found(format!(
                "Resource not found: {}",
                uri
            )));
        }
        let metadata = tokio::fs::metadata(target)
            .await
            .map_err(crate::errors::from_display)?;

        if metadata.len() > MAX_FILE_BYTES {
            return Err(crate::errors::file_too_large(format!(
                "Resource exceeds {} bytes",
                MAX_FILE_BYTES
            )));
        }

        let content = tokio::fs::read_to_string(target)
            .await
            .map_err(crate::errors::from_display)?;

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(content, uri)],
        })
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
            .read_resource_by_uri(
                "typstlab://rules",
                tokio_util::sync::CancellationToken::new(),
            )
            .await
            .expect("read resource");
        let content = &res.contents[0];
        let text = match content {
            ResourceContents::TextResourceContents { text, .. } => text,
            _ => panic!("expected text content"),
        };
        assert!(
            text.contains("\"path\":\"rules/a.md\""),
            "listing should include relative path"
        );
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
            .read_resource_by_uri(
                "typstlab://docs",
                tokio_util::sync::CancellationToken::new(),
            )
            .await
            .expect("read resource");
        let content = &res.contents[0];
        let text = match content {
            ResourceContents::TextResourceContents { text, .. } => text,
            _ => panic!("expected text content"),
        };
        assert!(text.contains("\"name\":\"b.md\""));
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
}
