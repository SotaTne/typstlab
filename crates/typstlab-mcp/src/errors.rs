use rmcp::ErrorData;
use serde_json::{Value, json};
use std::fmt::Display;
use typstlab_core::error::TypstlabError;

// 標準エラーコード定数（DESIGN.md 5.10.6準拠）
pub const PATH_ESCAPE: &str = "PATH_ESCAPE";
pub const INVALID_INPUT: &str = "INVALID_INPUT";
pub const NOT_FOUND: &str = "NOT_FOUND";
pub const FILE_TOO_LARGE: &str = "FILE_TOO_LARGE";
pub const BUILD_FAILED: &str = "BUILD_FAILED";
pub const TYPST_NOT_RESOLVED: &str = "TYPST_NOT_RESOLVED";
pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";

// 旧コード（v0.3で削除予定）
#[deprecated(since = "0.2.0", note = "Use NOT_FOUND instead")]
pub const PROJECT_NOT_FOUND: &str = "PROJECT_NOT_FOUND";

#[deprecated(since = "0.2.0", note = "Use PATH_ESCAPE instead")]
pub const PROJECT_PATH_ESCAPE: &str = "PROJECT_PATH_ESCAPE";

#[deprecated(since = "0.2.0", note = "Use FILE_TOO_LARGE instead")]
pub const PAPER_NOT_FOUND: &str = "PAPER_NOT_FOUND";

pub fn invalid_params(message: impl Into<String>) -> ErrorData {
    ErrorData::invalid_params(message.into(), None)
}

pub fn internal_error(message: impl Into<String>) -> ErrorData {
    error_with_code(INTERNAL_ERROR, message)
}

pub fn resource_not_found(message: impl Into<String>) -> ErrorData {
    ErrorData::resource_not_found(message.into(), None)
}

// 標準コード用ヘルパー関数
pub fn invalid_input(message: impl Into<String>) -> ErrorData {
    error_with_code(INVALID_INPUT, message)
}

pub fn path_escape(message: impl Into<String>) -> ErrorData {
    error_with_code(PATH_ESCAPE, message)
}

pub fn not_found(message: impl Into<String>) -> ErrorData {
    error_with_code(NOT_FOUND, message)
}

pub fn file_too_large(message: impl Into<String>) -> ErrorData {
    error_with_code(FILE_TOO_LARGE, message)
}

pub fn build_failed(message: impl Into<String>) -> ErrorData {
    error_with_code(BUILD_FAILED, message)
}

pub fn typst_not_resolved(message: impl Into<String>) -> ErrorData {
    error_with_code(TYPST_NOT_RESOLVED, message)
}

pub fn error_with_code(code: &str, message: impl Into<String>) -> ErrorData {
    ErrorData::internal_error(
        message.into(),
        Some(json!({
            "code": code
        })),
    )
}

pub fn error_with_data(code: &str, message: impl Into<String>, data: Value) -> ErrorData {
    let mut payload = json!({
        "code": code
    });
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("details".to_string(), data);
    }
    ErrorData::internal_error(message.into(), Some(payload))
}

pub fn from_core_error(error: TypstlabError) -> ErrorData {
    match error {
        TypstlabError::ProjectNotFound => not_found("Project not found"),
        TypstlabError::ProjectPathEscape { path } => error_with_data(
            PATH_ESCAPE, // PROJECT_PATH_ESCAPE → PATH_ESCAPE
            format!("Path resolves outside project root: {}", path.display()),
            json!({"path": path.display().to_string()}),
        ),
        TypstlabError::PaperNotFound(id) => not_found(format!("Paper not found: {}", id)),
        TypstlabError::TypstNotResolved { required_version } => {
            typst_not_resolved(format!("Typst {} is not resolved", required_version))
        }
        TypstlabError::BuildFailed(msg) => build_failed(msg),
        _ => internal_error(error.to_string()),
    }
}

pub fn from_display(error: impl Display) -> ErrorData {
    internal_error(format!("{}", error))
}

#[allow(dead_code)]
fn project_not_found(paper_id: Option<String>) -> ErrorData {
    let message = paper_id
        .map(|id| format!("Project not found for paper_id={}", id))
        .unwrap_or_else(|| "Project not found".to_string());
    not_found(message)
}
