pub mod tools;
pub mod utils;

use std::path::PathBuf;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{ErrorData as McpError, schemars, serve_server, tool, tool_router};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PaperId {
    #[schemars(description = "The ID of the paper to build and render.")]
    pub id: String,
}

pub struct TypstlabServer {
    project_root: PathBuf,
}

impl TypstlabServer {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }
}

#[tool_router(server_handler)]
impl TypstlabServer {
    #[tool(
        description = "Get the current typstlab project status, including toolchain, docs path and cache, papers, templates, and dist paths."
    )]
    async fn status(&self) -> Result<CallToolResult, McpError> {
        let root = self.project_root.clone();

        tokio::task::spawn_blocking(move || {
            let ctx = utils::bootstrap_context(root).map_err(utils::internal_error)?;
            tools::status::execute(ctx).map_err(utils::internal_error)
        })
        .await
        .map_err(|error| McpError::internal_error(error.to_string(), None))?
    }

    #[tool(
        description = "Build a paper by ID, returning the base64-encoded PNG image of each page."
    )]
    async fn build_and_render(
        &self,
        Parameters(PaperId { id }): Parameters<PaperId>,
    ) -> Result<CallToolResult, McpError> {
        let root = self.project_root.clone();

        tokio::task::spawn_blocking(move || {
            let ctx = utils::bootstrap_context(root).map_err(utils::internal_error)?;
            tools::build_and_render::execute(ctx, id).map_err(utils::internal_error)
        })
        .await
        .map_err(|error| McpError::internal_error(error.to_string(), None))?
    }
}

pub async fn serve_stdio(project_root: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let service = serve_server(TypstlabServer::new(project_root), rmcp::transport::stdio()).await?;

    service.waiting().await?;
    Ok(())
}
