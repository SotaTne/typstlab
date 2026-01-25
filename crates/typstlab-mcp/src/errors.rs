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
    ErrorData::internal_error(message.into(), Some(json!({ "code": INTERNAL_ERROR })))
}

pub fn resource_not_found(message: impl Into<String>) -> ErrorData {
    ErrorData {
        code: rmcp::model::ErrorCode(-32002), // ResourceNotFound
        message: message.into().into(),
        data: Some(json!({ "code": NOT_FOUND })),
    }
}

// 標準コード用ヘルパー関数
pub fn request_cancelled() -> ErrorData {
    ErrorData {
        code: rmcp::model::ErrorCode(-32800),
        message: "Request cancelled".into(),
        data: None,
    }
}

pub fn invalid_input(message: impl Into<String>) -> ErrorData {
    ErrorData {
        code: rmcp::model::ErrorCode(-32602), // InvalidParams
        message: message.into().into(),
        data: Some(json!({ "code": INVALID_INPUT })),
    }
}

pub fn path_escape(message: impl Into<String>) -> ErrorData {
    ErrorData {
        code: rmcp::model::ErrorCode(-32001),
        message: message.into().into(),
        data: Some(json!({ "code": PATH_ESCAPE })),
    }
}

pub fn not_found(message: impl Into<String>) -> ErrorData {
    resource_not_found(message)
}

pub fn file_too_large(message: impl Into<String>) -> ErrorData {
    ErrorData {
        code: rmcp::model::ErrorCode(-32003),
        message: message.into().into(),
        data: Some(json!({ "code": FILE_TOO_LARGE })),
    }
}

pub fn build_failed(message: impl Into<String>) -> ErrorData {
    ErrorData {
        code: rmcp::model::ErrorCode(-32004),
        message: message.into().into(),
        data: Some(json!({ "code": BUILD_FAILED })),
    }
}

pub fn typst_not_resolved(message: impl Into<String>) -> ErrorData {
    ErrorData {
        code: rmcp::model::ErrorCode(-32005),
        message: message.into().into(),
        data: Some(json!({ "code": TYPST_NOT_RESOLVED })),
    }
}

pub fn error_with_code(code: &str, message: impl Into<String>) -> ErrorData {
    // For backward compatibility or fallback
    ErrorData::internal_error(
        message.into(),
        Some(json!({
            "code": code
        })),
    )
}

pub fn error_with_data(code: &str, message: impl Into<String>, data: Value) -> ErrorData {
    // Try to map known codes to standard JSON-RPC codes if possible, otherwise use InternalError
    let rpc_code = match code {
        PATH_ESCAPE => -32001,
        NOT_FOUND => -32002,
        FILE_TOO_LARGE => -32003,
        BUILD_FAILED => -32004,
        TYPST_NOT_RESOLVED => -32005,
        INVALID_INPUT => -32602,
        _ => -32603,
    };

    let mut payload = json!({
        "code": code
    });
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("details".to_string(), data);
    }

    ErrorData {
        code: rmcp::model::ErrorCode(rpc_code),
        message: message.into().into(),
        data: Some(payload),
    }
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
