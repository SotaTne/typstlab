use rmcp::model::CallToolResult;
use serde::Serialize;
use std::path::PathBuf;
use typstlab_app::{AppContext, StatusAction, StatusOutput, StatusWarning};
use typstlab_proto::Action;

#[derive(Debug, Serialize)]
struct McpStatusResult {
    status: StatusOutput,
    warnings: Vec<McpStatusWarning>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum McpStatusWarning {
    #[serde(rename = "papers_dir_not_found")]
    Papers { path: PathBuf },
    #[serde(rename = "templates_dir_not_found")]
    Templates { path: PathBuf },
    #[serde(rename = "dist_dir_not_found")]
    Dist { path: PathBuf },
}

pub fn execute(ctx: AppContext) -> Result<CallToolResult, String> {
    let action = StatusAction::new(ctx.loaded_project, ctx.typst, ctx.docs);
    let mut warnings = Vec::new();
    let status = action
        .run(&mut |_| {}, &mut |warning| warnings.push(warning))
        .map_err(|errors| format_status_errors(&errors))?;

    let result = McpStatusResult {
        status,
        warnings: warnings.into_iter().map(McpStatusWarning::from).collect(),
    };
    let value = serde_json::to_value(result)
        .map_err(|error| format!("failed to serialize status result: {}", error))?;

    Ok(CallToolResult::structured(value))
}

fn format_status_errors(errors: &[typstlab_app::StatusError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

impl From<StatusWarning> for McpStatusWarning {
    fn from(warning: StatusWarning) -> Self {
        match warning {
            StatusWarning::PapersDirNotFound(path) => Self::Papers { path },
            StatusWarning::TemplatesDirNotFound(path) => Self::Templates { path },
            StatusWarning::DistDirNotFound(path) => Self::Dist { path },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_status_warning_serializes_stable_kind() {
        let warning =
            McpStatusWarning::from(StatusWarning::PapersDirNotFound(PathBuf::from("papers")));

        let value = serde_json::to_value(warning).unwrap();

        assert_eq!(
            value,
            json!({
                "kind": "papers_dir_not_found",
                "path": "papers",
            })
        );
    }
}
