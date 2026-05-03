use thiserror::Error;

#[derive(Debug, Error)]
pub enum HtmlParseError {
    #[error("html tokenizer failed: {0}")]
    Tokenizer(String),

    #[error("unsupported html tag: {0}")]
    UnsupportedTag(String),

    #[error("html stack is unexpectedly empty")]
    EmptyStack,

    #[error("failed to convert html to markdown document: {0}")]
    MarkdownDocument(#[from] HtmlToMarkdownError),
}

#[derive(Debug, Error)]
pub enum HtmlToMarkdownError {
    #[error("docs route resolution failed: {0}")]
    Route(String),

    #[error("html tag cannot be converted to markdown document here: {0:?}")]
    UnsupportedTag(crate::docs_parser::html::HtmlTag),

    #[error("invalid table structure: {0}")]
    InvalidTable(String),
}

#[derive(Debug, Error)]
pub enum HtmlRenderError {
    #[error("failed to parse html: {0}")]
    Parse(#[from] HtmlParseError),

    #[error("failed to convert html to markdown document: {0}")]
    MarkdownDocument(#[from] HtmlToMarkdownError),
}
