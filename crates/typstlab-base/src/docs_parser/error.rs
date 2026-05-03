use thiserror::Error;

#[derive(Debug, Error)]
pub enum DocsRenderError {
    #[error("failed to parse docs.json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("failed to create rendered docs tempdir: {0}")]
    TempDir(#[source] std::io::Error),

    #[error("failed to write rendered docs: {0}")]
    Io(#[from] std::io::Error),

    #[error("sink failed: {0}")]
    Sink(String),

    #[error("route must start with /DOCS-BASE/: {0}")]
    MissingPrefix(String),

    #[error("route must not be empty")]
    EmptyRoute,

    #[error("path must not contain absolute or rooted path: {0}")]
    RootedPath(String),

    #[error("path must not contain parent directory traversal: {0}")]
    PathTraversal(String),

    #[error("docs body render failed: {0}")]
    Body(String),

    #[error("html render failed: {0}")]
    Html(#[from] crate::docs_parser::html::HtmlRenderError),
}
