pub mod ast;
pub mod entities;
pub mod error;
pub mod markdown;
pub mod parser;
pub mod render;

pub use ast::{Html, HtmlAttrs, HtmlElement, HtmlNode, HtmlTag, HtmlTree};
pub use entities::decode_entities;
pub use error::{HtmlParseError, HtmlRenderError, HtmlToMarkdownError};
pub use markdown::{
    MarkdownContext, ToMarkdownDocument, html_tree_to_markdown_document,
    html_tree_to_markdown_document_with_context,
};
pub use parser::parse_html;
pub use render::{html_to_markdown, html_to_markdown_with_source_route};
