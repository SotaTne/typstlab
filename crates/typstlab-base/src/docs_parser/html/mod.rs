pub mod ast;
pub mod entities;
pub mod error;
pub mod markdown;
pub mod parser;
pub mod render;

pub use ast::{Html, HtmlAttrs, HtmlElement, HtmlNode, HtmlTag, HtmlTree};
pub use entities::decode_entities;
pub use error::{HtmlParseError, HtmlRenderError, HtmlToMarkdownError};
pub use markdown::{ToMarkdownDocument, html_tree_to_markdown_document};
pub use parser::parse_html;
pub use render::html_to_markdown;
