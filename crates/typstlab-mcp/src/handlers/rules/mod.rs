//! Rules handlers module

mod browse;
mod get;
mod list;
mod search;
mod types;

// Re-export types
pub use types::*;

// Re-export helper functions used externally
pub(crate) use browse::rules_browse_items;

use crate::handlers::{Safety, ToolExt};
use crate::server::TypstlabServer;
use futures_util::FutureExt;
use rmcp::{
    handler::server::common::FromContextPart,
    handler::server::router::tool::{ToolRoute, ToolRouter},
    handler::server::wrapper::Parameters,
    model::*,
};
use std::borrow::Cow;

pub struct RulesTool;

impl RulesTool {
    pub fn into_router(self) -> ToolRouter<TypstlabServer> {
        ToolRouter::new()
            .with_route(ToolRoute::new_dyn(Self::rules_browse_attr(), |mut ctx| {
                let server = ctx.service;
                let args_res = Parameters::<RulesBrowseArgs>::from_context_part(&mut ctx);
                async move {
                    let Parameters(args) = args_res?;
                    browse::rules_browse(server, args).await
                }
                .boxed()
            }))
            .with_route(ToolRoute::new_dyn(Self::rules_search_attr(), |mut ctx| {
                let server = ctx.service;
                let args_res = Parameters::<RulesSearchArgs>::from_context_part(&mut ctx);
                async move {
                    let Parameters(args) = args_res?;
                    search::rules_search(server, args).await
                }
                .boxed()
            }))
            // Note: rules_get is not publicized as a tool (DESIGN.md 5.10.1)
            // Content retrieval should use read_resource (typstlab://rules/*)
            .with_route(ToolRoute::new_dyn(Self::rules_page_attr(), |mut ctx| {
                let server = ctx.service;
                let args_res = Parameters::<RulesPageArgs>::from_context_part(&mut ctx);
                async move {
                    let Parameters(args) = args_res?;
                    get::rules_page(server, args).await
                }
                .boxed()
            }))
            .with_route(ToolRoute::new_dyn(Self::rules_list_attr(), |mut ctx| {
                let server = ctx.service;
                let args_res = Parameters::<RulesListArgs>::from_context_part(&mut ctx);
                async move {
                    let Parameters(args) = args_res?;
                    list::rules_list(server, args).await
                }
                .boxed()
            }))
    }

    fn rules_browse_attr() -> Tool {
        Tool::new(
            Cow::Borrowed("rules_browse"),
            "List files and directories under rules paths",
            rmcp::handler::server::common::schema_for_type::<RulesBrowseArgs>(),
        )
        .with_safety(Safety {
            network: false,
            reads: true,
            writes: false,
            writes_sot: false,
        })
    }

    fn rules_search_attr() -> Tool {
        Tool::new(
            Cow::Borrowed("rules_search"),
            "Search through markdown files in rules directories",
            rmcp::handler::server::common::schema_for_type::<RulesSearchArgs>(),
        )
        .with_safety(Safety {
            network: false,
            reads: true,
            writes: false,
            writes_sot: false,
        })
    }

    // rules_get_attr is intentionally removed (DESIGN.md 5.10.1)
    // Use read_resource (typstlab://rules/*) for content retrieval

    fn rules_page_attr() -> Tool {
        Tool::new(
            Cow::Borrowed("rules_page"),
            "Read a slice of content from a rule file",
            rmcp::handler::server::common::schema_for_type::<RulesPageArgs>(),
        )
        .with_safety(Safety {
            network: false,
            reads: true,
            writes: false,
            writes_sot: false,
        })
    }

    fn rules_list_attr() -> Tool {
        Tool::new(
            Cow::Borrowed("rules_list"),
            "List all rule files in the project",
            rmcp::handler::server::common::schema_for_type::<RulesListArgs>(),
        )
        .with_safety(Safety {
            network: false,
            reads: true,
            writes: false,
            writes_sot: false,
        })
    }

    // テスト用: ハンドラ関数を公開
    pub async fn rules_browse(
        server: &TypstlabServer,
        args: RulesBrowseArgs,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        browse::rules_browse(server, args).await
    }

    pub async fn rules_search(
        server: &TypstlabServer,
        args: RulesSearchArgs,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        search::rules_search(server, args).await
    }

    // rules_get kept for internal use and testing
    // Not exposed as a public tool (use read_resource instead)
    pub async fn rules_get(
        server: &TypstlabServer,
        args: RulesGetArgs,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        get::rules_get(server, args).await
    }

    pub async fn rules_page(
        server: &TypstlabServer,
        args: RulesPageArgs,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        get::rules_page(server, args).await
    }

    pub async fn rules_list(
        server: &TypstlabServer,
        args: RulesListArgs,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        list::rules_list(server, args).await
    }
}

#[cfg(test)]
mod tests;
