use thiserror::Error;

use super::ast::Span;

#[derive(Debug, Error)]
pub enum TemplateEngineError {
    #[error("missing template parameter: {0}")]
    MissingParam(String),
    #[error("unknown template parameter: {0}")]
    UnknownParam(String),
    #[error("unknown template default source '{source_name}' for parameter '{param}'")]
    UnknownDefaultSource { param: String, source_name: String },
    #[error("unknown template builtin: {0}")]
    UnknownBuiltin(String),
    #[error("invalid template parameter '{name}': expected {expected}")]
    InvalidParam { name: String, expected: String },
    #[error("template value cannot be rendered as display text")]
    NonDisplayableValue,
    #[error("template parse error: {kind} at byte range {}..{}", span.start, span.end)]
    Parse { kind: ParseErrorKind, span: Span },
    #[error("{label}: {error}")]
    ParseInSource {
        label: String,
        #[source]
        error: Box<Self>,
    },
    #[error("template render error: {0}")]
    Render(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseErrorKind {
    #[error("unclosed template tag")]
    UnclosedTag,
    #[error("empty template tag")]
    EmptyTag,
    #[error("empty parameter name")]
    EmptyParamName,
    #[error("invalid parameter name")]
    InvalidParamName,
}
