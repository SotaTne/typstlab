use rmcp::{ErrorData as McpError, schemars, serde};
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct DocsBrowseArgs {
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct DocsSearchArgs {
    pub query: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct DocsGetArgs {
    pub path: String,
}

pub(crate) async fn resolve_docs_path(
    project_root: &Path,
    docs_root: &Path,
    requested: &Path,
) -> Result<PathBuf, McpError> {
    use crate::handlers::common::{ops::check_entry_safety, path::resolve_safe_path};
    // First, perform the standard path validation relative to docs_root
    let resolved = resolve_safe_path(docs_root, requested).await?;

    // Additional defense: ensure the resolved path stays under the project root even if docs_root
    // itself is a symlink pointing outside. Only check if path exists.
    if resolved.exists() {
        check_entry_safety(&resolved, project_root)?;
    }

    Ok(resolved)
}
