use crate::errors;
use crate::handlers::{Safety, ToolExt};
use crate::server::TypstlabServer;
use futures_util::FutureExt;
use rmcp::{
    ErrorData as McpError,
    handler::server::common::FromContextPart,
    handler::server::router::tool::{ToolRoute, ToolRouter},
    handler::server::wrapper::Parameters,
    model::*,
    schemars, serde,
};
use serde_json::json;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

pub struct DocsTool;

use typstlab_core::config::consts::search::{
    MAX_FILE_BYTES, MAX_MATCHES, MAX_MATCHES_PER_FILE, MAX_SCAN_FILES,
};

impl DocsTool {
    pub fn into_router(self) -> ToolRouter<TypstlabServer> {
        ToolRouter::new()
            .with_route(ToolRoute::new_dyn(Self::docs_browse_attr(), |mut ctx| {
                let server = ctx.service;
                let args_res = Parameters::<DocsBrowseArgs>::from_context_part(&mut ctx);
                async move {
                    let Parameters(args) = args_res?;
                    Self::docs_browse(server, args).await
                }
                .boxed()
            }))
            .with_route(ToolRoute::new_dyn(Self::docs_search_attr(), |mut ctx| {
                let server = ctx.service;
                let args_res = Parameters::<DocsSearchArgs>::from_context_part(&mut ctx);
                async move {
                    let Parameters(args) = args_res?;
                    Self::docs_search(server, args).await
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
            "Search documentation files",
            rmcp::handler::server::common::schema_for_type::<DocsSearchArgs>(),
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
        Self::docs_browse(server, args).await
    }

    pub async fn test_docs_search(
        server: &TypstlabServer,
        args: DocsSearchArgs,
    ) -> Result<CallToolResult, McpError> {
        Self::docs_search(server, args).await
    }

    async fn docs_browse(
        server: &TypstlabServer,
        args: DocsBrowseArgs,
    ) -> Result<CallToolResult, McpError> {
        let docs_root = server.context.project_root.join(".typstlab/kb/typst/docs");

        // DESIGN.md 5.10.5: browse系は {items: [], missing: bool, truncated?: bool}
        // docsルート自体が存在しない場合
        if !docs_root.exists() {
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string(&json!({
                    "items": [],
                    "truncated": false,
                    "missing": true,
                }))
                .map_err(errors::from_display)?,
            )]));
        }

        let path = args.path.unwrap_or_default();
        let target = resolve_docs_path(&docs_root, Path::new(&path)).await?;

        // 存在しないパスもmissing=trueで返す（エラーにしない）
        if !target.exists() || !target.is_dir() {
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string(&json!({
                    "items": [],
                    "truncated": false,
                    "missing": true,
                }))
                .map_err(errors::from_display)?,
            )]));
        }

        let mut items = Vec::new();
        let mut dir = fs::read_dir(&target).await.map_err(errors::from_display)?;
        while let Some(entry) = dir.next_entry().await.map_err(errors::from_display)? {
            let file_type = entry.file_type().await.map_err(errors::from_display)?;
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
            } else {
                if entry_path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                    continue;
                }
                "file"
            };
            items.push(json!({
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
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&json!({ "items": items })).map_err(errors::from_display)?,
        )]))
    }

    async fn docs_search(
        server: &TypstlabServer,
        args: DocsSearchArgs,
    ) -> Result<CallToolResult, McpError> {
        // Validate query before processing
        let trimmed_query = args.query.trim();
        if trimmed_query.is_empty() {
            return Err(errors::invalid_input(
                "Search query cannot be empty or whitespace-only",
            ));
        }
        if trimmed_query.len() > 1000 {
            return Err(errors::invalid_input(
                "Search query too long (max 1000 characters)",
            ));
        }

        let docs_root = server.context.project_root.join(".typstlab/kb/typst/docs");
        // DESIGN.md 5.10.5: search系は常に { matches: [], truncated: bool, missing: bool }
        if !docs_root.exists() {
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string(&json!({
                    "matches": [],
                    "truncated": false,
                    "missing": true,
                }))
                .map_err(errors::from_display)?,
            )]));
        }

        let canonical_root = fs::canonicalize(&docs_root)
            .await
            .map_err(errors::from_display)?;
        let query_lowercase = trimmed_query.to_lowercase();
        let outcome = search_docs(&docs_root, &canonical_root, &query_lowercase).await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&json!({
                "matches": outcome.matches,
                "truncated": outcome.truncated,
            }))
            .map_err(errors::from_display)?,
        )]))
    }
}

struct DocsSearchOutcome {
    matches: Vec<serde_json::Value>,
    truncated: bool,
}

async fn search_docs(
    docs_root: &Path,
    canonical_root: &Path,
    query: &str,
) -> Result<DocsSearchOutcome, McpError> {
    let query = query.to_lowercase();
    let mut matches = Vec::new();
    let mut truncated = false;
    let mut scanned = 0usize;

    for entry in WalkDir::new(docs_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = match docs_search_entry_path(&entry, canonical_root).await? {
            Some(path) => path,
            None => continue,
        };

        // DESIGN.md 5.10.9: MAX_SCAN_FILES超過前にチェック
        if scanned >= MAX_SCAN_FILES {
            truncated = true;
            matches.clear();
            break;
        }

        scanned += 1;
        let file_matches = docs_matches_for_file(&path, docs_root, &query).await?;
        matches.extend(file_matches);
        if matches.len() >= MAX_MATCHES {
            matches.truncate(MAX_MATCHES);
            truncated = true; // DESIGN.md 5.10.9: MAX_MATCHES到達時はtruncated=true
            break;
        }
    }

    Ok(DocsSearchOutcome { matches, truncated })
}

async fn docs_search_entry_path(
    entry: &walkdir::DirEntry,
    canonical_root: &Path,
) -> Result<Option<std::path::PathBuf>, McpError> {
    let file_type = entry.file_type();
    if file_type.is_symlink() || !file_type.is_file() {
        return Ok(None);
    }

    let path = entry.path();
    if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
        return Ok(None);
    }

    let canonical_target = match fs::canonicalize(path).await {
        Ok(target) => target,
        Err(_) => return Ok(None),
    };
    if !canonical_target.starts_with(canonical_root) {
        return Ok(None);
    }

    let metadata = fs::metadata(path).await.map_err(errors::from_display)?;
    if metadata.len() > MAX_FILE_BYTES {
        return Ok(None);
    }

    Ok(Some(path.to_path_buf()))
}

async fn docs_matches_for_file(
    path: &Path,
    docs_root: &Path,
    query: &str,
) -> Result<Vec<serde_json::Value>, McpError> {
    let content = fs::read_to_string(path)
        .await
        .map_err(errors::from_display)?;
    let mut matches = Vec::new();

    for (line_index, line) in content.lines().enumerate() {
        if line.to_lowercase().contains(query) {
            let rel_path = path
                .strip_prefix(docs_root)
                .map_err(errors::from_display)?
                .to_string_lossy();
            matches.push(json!({
                "path": rel_path,
                "line": line_index + 1,
                "content": line.trim(),
            }));
            if matches.len() >= MAX_MATCHES_PER_FILE {
                break;
            }
        }
    }

    Ok(matches)
}

pub(crate) async fn resolve_docs_path(
    docs_root: &Path,
    requested: &Path,
) -> Result<PathBuf, McpError> {
    use crate::handlers::common::path::resolve_safe_path;
    resolve_safe_path(docs_root, requested).await
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct DocsBrowseArgs {
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct DocsSearchArgs {
    pub query: String,
}

#[cfg(test)]
mod tests;
