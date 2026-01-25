//! Rules関連の型定義と共通ヘルパー

use crate::errors;
use rmcp::{ErrorData as McpError, schemars, serde};
use std::path::{Component, Path, PathBuf};
use tokio::fs;
use typstlab_core::path::has_absolute_or_rooted_component;

// ==================== 型定義 ====================

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct RulesBrowseArgs {
    pub path: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct RulesGetArgs {
    pub path: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct RulesPageArgs {
    pub path: String,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct RulesListArgs {
    pub paper_id: Option<String>,
    #[serde(default = "default_true")]
    pub include_root: bool,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct RulesSearchArgs {
    pub query: String,
    pub paper_id: Option<String>,
    #[serde(default = "default_true")]
    pub include_root: bool,
}

pub(crate) fn default_true() -> bool {
    true
}

// ==================== 共通ヘルパー ====================

/// rulesパスを解決する
pub(crate) async fn resolve_rules_path(
    project_root: &Path,
    requested: &Path,
) -> Result<PathBuf, McpError> {
    use crate::handlers::common::path::resolve_safe_path;

    // ルール固有のパス構造バリデーション
    if has_absolute_or_rooted_component(requested) {
        return Err(errors::invalid_params("Path cannot be absolute or rooted"));
    }

    let components: Vec<Component> = requested.components().collect();
    if components.iter().any(|c| matches!(c, Component::ParentDir)) {
        return Err(errors::invalid_params("Path cannot contain .."));
    }

    let first = components.first();
    let is_rules = first == Some(&Component::Normal("rules".as_ref()));
    let is_papers = first == Some(&Component::Normal("papers".as_ref()));

    if !is_rules && !is_papers {
        return Err(errors::invalid_params(
            "Path must start with rules/ or papers/<paper_id>/rules",
        ));
    }

    if is_papers
        && (components.len() < 3 || components.get(2) != Some(&Component::Normal("rules".as_ref())))
    {
        return Err(errors::invalid_params(
            "Path must be within papers/<paper_id>/rules",
        ));
    }

    // 共通パス解決処理（セキュリティチェック含む）
    resolve_safe_path(project_root, requested).await
}

/// 検索ディレクトリを収集する
pub(crate) fn collect_search_dirs(
    project_root: &Path,
    args: &RulesSearchArgs,
) -> Vec<(std::path::PathBuf, &'static str)> {
    let mut search_dirs = Vec::new();
    if args.include_root {
        search_dirs.push((project_root.join("rules"), "root"));
    }

    if let Some(paper_id) = args.paper_id.as_ref() {
        search_dirs.push((
            project_root.join("papers").join(paper_id).join("rules"),
            "paper",
        ));
    }

    search_dirs
}

/// ファイルサイズを強制する
pub(crate) async fn enforce_rules_file_size(path: &Path) -> Result<(), McpError> {
    use typstlab_core::config::consts::search::MAX_FILE_BYTES;

    let metadata = fs::metadata(path).await.map_err(errors::from_display)?;
    if metadata.len() > MAX_FILE_BYTES {
        return Err(errors::error_with_code(
            errors::FILE_TOO_LARGE,
            format!(
                "File exceeds maximum allowed size of {} bytes",
                MAX_FILE_BYTES
            ),
        ));
    }
    Ok(())
}
