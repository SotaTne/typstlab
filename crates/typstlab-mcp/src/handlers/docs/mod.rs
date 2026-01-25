use crate::handlers::{Safety, ToolExt};
use crate::server::TypstlabServer;
use futures_util::FutureExt;
use rmcp::{
    ErrorData as McpError,
    handler::server::common::FromContextPart,
    handler::server::router::tool::{ToolRoute, ToolRouter},
    handler::server::wrapper::Parameters,
    model::*,
};
use std::borrow::Cow;
use tokio_util::sync::CancellationToken;

mod browse;
mod get;
mod search;
mod types;

pub use types::*;

pub struct DocsTool;

impl DocsTool {
    pub fn into_router(self) -> ToolRouter<TypstlabServer> {
        ToolRouter::new()
            .with_route(ToolRoute::new_dyn(Self::docs_browse_attr(), |mut ctx| {
                let server = ctx.service;
                let token = ctx.request_context.ct.clone(); // Use request token
                let args_res = Parameters::<DocsBrowseArgs>::from_context_part(&mut ctx);
                async move {
                    let Parameters(args) = args_res?;
                    browse::docs_browse(server, args, token).await
                }
                .boxed()
            }))
            .with_route(ToolRoute::new_dyn(Self::docs_search_attr(), |mut ctx| {
                let server = ctx.service;
                let token = ctx.request_context.ct.clone();
                let args_res = Parameters::<DocsSearchArgs>::from_context_part(&mut ctx);
                async move {
                    let Parameters(args) = args_res?;
                    search::docs_search(server, args, token).await
                }
                .boxed()
            }))
            .with_route(ToolRoute::new_dyn(Self::docs_get_attr(), |mut ctx| {
                let server = ctx.service;
                let args_res = Parameters::<DocsGetArgs>::from_context_part(&mut ctx);
                async move {
                    let Parameters(args) = args_res?;
                    get::docs_get(server, args).await
                }
                .boxed()
            }))
    }

    fn docs_browse_attr() -> Tool {
        Tool::new(
            Cow::Borrowed("docs_browse"),
            "Browse documentation directory structure",
            rmcp::handler::server::common::schema_for_type::<DocsBrowseArgs>(),
        )
        .with_safety(Safety {
            network: false,
            reads: true,
            writes: false,
            writes_sot: false,
        })
    }

    fn docs_search_attr() -> Tool {
        Tool::new(
            Cow::Borrowed("docs_search"),
            "Search documentation files (line-based substring match, case-insensitive)",
            rmcp::handler::server::common::schema_for_type::<DocsSearchArgs>(),
        )
        .with_safety(Safety {
            network: false,
            reads: true,
            writes: false,
            writes_sot: false,
        })
    }

    fn docs_get_attr() -> Tool {
        Tool::new(
            Cow::Borrowed("docs_get"),
            "Get the content of a documentation file (identical validation/read_resource path)",
            rmcp::handler::server::common::schema_for_type::<DocsGetArgs>(),
        )
        .with_safety(Safety {
            network: false,
            reads: true,
            writes: false,
            writes_sot: false,
        })
    }

    // テスト用: ハンドラ関数をpublicラッパー経由で公開
    pub async fn test_docs_browse(
        server: &TypstlabServer,
        args: DocsBrowseArgs,
    ) -> Result<CallToolResult, McpError> {
        browse::docs_browse(server, args, CancellationToken::new()).await
    }

    pub async fn test_docs_search(
        server: &TypstlabServer,
        args: DocsSearchArgs,
    ) -> Result<CallToolResult, McpError> {
        search::docs_search(server, args, CancellationToken::new()).await
    }

    // For compatibility if needed, but not heavily used in tests directly other than tool calls
    pub async fn docs_browse(
        server: &TypstlabServer,
        args: DocsBrowseArgs,
        token: CancellationToken,
    ) -> Result<CallToolResult, McpError> {
        browse::docs_browse(server, args, token).await
    }

    pub async fn docs_search(
        server: &TypstlabServer,
        args: DocsSearchArgs,
        token: CancellationToken,
    ) -> Result<CallToolResult, McpError> {
        search::docs_search(server, args, token).await
    }

    pub async fn docs_get(
        server: &TypstlabServer,
        args: DocsGetArgs,
    ) -> Result<CallToolResult, McpError> {
        get::docs_get(server, args).await
    }
}

#[cfg(test)]
mod tests;
