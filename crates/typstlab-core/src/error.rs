use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TypstlabError {
    // Project errors
    #[error("PROJECT_NOT_FOUND: typstlab.toml not found in current or parent directories")]
    ProjectNotFound,

    #[error("PROJECT_INVALID_STRUCTURE: {0}")]
    ProjectInvalidStructure(String),

    #[error("PROJECT_CONFIG_INVALID: failed to parse typstlab.toml: {0}")]
    ProjectConfigInvalid(String),

    #[error("PROJECT_PATH_ESCAPE: path '{path}' resolves outside project root")]
    ProjectPathEscape { path: PathBuf },

    // Paper errors
    #[error("PAPER_NOT_FOUND: paper '{0}' not found")]
    PaperNotFound(String),

    #[error("PAPER_CONFIG_INVALID: failed to parse paper.toml for '{paper_id}': {reason}")]
    PaperConfigInvalid { paper_id: String, reason: String },

    #[error(
        "PAPER_ID_MISMATCH: paper.toml id '{toml_id}' does not match directory name '{dir_name}'"
    )]
    PaperIdMismatch { toml_id: String, dir_name: String },

    #[error("PAPER_MAIN_NOT_FOUND: main.typ not found for paper '{0}'")]
    PaperMainNotFound(String),

    // Config errors
    #[error("CONFIG_PARSE_ERROR: {0}")]
    ConfigParseError(String),

    #[error("CONFIG_INVALID_VALUE: {field}: {reason}")]
    ConfigInvalidValue { field: String, reason: String },

    // Typst errors
    #[error("TYPST_NOT_RESOLVED: Typst {required_version} is not resolved")]
    TypstNotResolved { required_version: String },

    #[error("TYPST_VERSION_MISMATCH: required {required}, found {found}")]
    TypstVersionMismatch { required: String, found: String },

    #[error("TYPST_INSTALL_FAILED: {0}")]
    TypstInstallFailed(String),

    #[error("TYPST_EXEC_FAILED: {0}")]
    TypstExecFailed(String),

    // Build errors
    #[error("BUILD_FAILED: {0}")]
    BuildFailed(String),

    #[error("BUILD_MISSING_DEPENDENCY: {0}")]
    BuildMissingDependency(String),

    // Network errors
    #[error("NETWORK_POLICY_VIOLATION: network access denied by policy (network = '{policy}')")]
    NetworkPolicyViolation { policy: String },

    #[error("NETWORK_FETCH_FAILED: {0}")]
    NetworkFetchFailed(String),

    // State errors
    #[error("STATE_READ_ERROR: failed to read state.json: {0}")]
    StateReadError(String),

    #[error("STATE_WRITE_ERROR: failed to write state.json: {0}")]
    StateWriteError(String),

    #[error("STATE_INVALID_SCHEMA: unknown schema version '{0}'")]
    StateInvalidSchema(String),

    // Refs errors
    #[error("REFS_SET_NOT_FOUND: refs set '{0}' not found")]
    RefsSetNotFound(String),

    #[error("REFS_FETCH_FAILED: {0}")]
    RefsFetchFailed(String),

    #[error("REFS_KEY_COLLISION: key '{key}' exists in multiple sets: {sets}")]
    RefsKeyCollision { key: String, sets: String },

    // Layout errors
    #[error("LAYOUT_NOT_FOUND: layout '{0}' not found")]
    LayoutNotFound(String),

    #[error("LAYOUT_INVALID: {0}")]
    LayoutInvalid(String),

    // IO errors
    #[error("IO_ERROR: {0}")]
    IoError(#[from] std::io::Error),

    // Generic errors
    #[error("{0}")]
    Generic(String),
}

impl From<serde_json::Error> for TypstlabError {
    fn from(err: serde_json::Error) -> Self {
        TypstlabError::Generic(format!("JSON error: {}", err))
    }
}

impl From<crate::template::error::TemplateError> for TypstlabError {
    fn from(err: crate::template::error::TemplateError) -> Self {
        TypstlabError::Generic(format!("Template error: {}", err))
    }
}

pub type Result<T> = std::result::Result<T, TypstlabError>;
