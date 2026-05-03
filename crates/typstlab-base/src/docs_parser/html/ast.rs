use super::error::{HtmlParseError, HtmlRenderError, HtmlToMarkdownError};
use super::markdown::ToMarkdownDocument;
use super::{parser, render};
use crate::docs_parser::md::Document;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Html {
    pub root: HtmlTree,
}

impl Html {
    pub fn parse(input: &str) -> Result<Self, HtmlParseError> {
        let tree = parser::parse_html(input)?;
        Ok(Self { root: tree })
    }

    pub fn to_document(&self) -> Document {
        self.root
            .to_markdown_document()
            .expect("html to markdown conversion must succeed")
    }

    pub fn to_markdown(&self) -> Result<String, HtmlRenderError> {
        render::html_to_markdown(self)
    }

    pub fn to_markdown_with_source_route(
        &self,
        source_route: &str,
    ) -> Result<String, HtmlRenderError> {
        render::html_to_markdown_with_source_route(self, source_route)
    }
}

impl<'de> Deserialize<'de> for Html {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let input = String::deserialize(deserializer)?;
        Self::parse(&input).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<HtmlTree> for Document {
    type Error = HtmlToMarkdownError;

    fn try_from(value: HtmlTree) -> Result<Self, Self::Error> {
        value.to_markdown_document()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HtmlTree {
    pub children: Vec<HtmlNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum HtmlNode {
    Element(HtmlElement),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HtmlElement {
    pub tag: HtmlTag,
    pub attrs: HtmlAttrs,
    pub children: Vec<HtmlNode>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct HtmlAttrs {
    pub href: Option<String>,
    pub src: Option<String>,
    pub alt: Option<String>,
    pub title: Option<String>,
    pub class: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HtmlTag {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    P,
    A,
    Code,
    Pre,
    Ul,
    Ol,
    Li,
    Strong,
    Em,
    Blockquote,
    Br,
    Img,
    Table,
    Thead,
    Tbody,
    Tr,
    Th,
    Td,
    Div,
    Span,
    Sup,
    Kbd,
    Details,
    Summary,
}

impl HtmlTag {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "h1" => Some(Self::H1),
            "h2" => Some(Self::H2),
            "h3" => Some(Self::H3),
            "h4" => Some(Self::H4),
            "h5" => Some(Self::H5),
            "h6" => Some(Self::H6),
            "p" => Some(Self::P),
            "a" => Some(Self::A),
            "code" => Some(Self::Code),
            "pre" => Some(Self::Pre),
            "ul" => Some(Self::Ul),
            "ol" => Some(Self::Ol),
            "li" => Some(Self::Li),
            "strong" | "b" => Some(Self::Strong),
            "em" | "i" => Some(Self::Em),
            "blockquote" => Some(Self::Blockquote),
            "br" => Some(Self::Br),
            "img" => Some(Self::Img),
            "table" => Some(Self::Table),
            "thead" => Some(Self::Thead),
            "tbody" => Some(Self::Tbody),
            "tr" => Some(Self::Tr),
            "th" => Some(Self::Th),
            "td" => Some(Self::Td),
            "div" => Some(Self::Div),
            "span" => Some(Self::Span),
            "sup" => Some(Self::Sup),
            "kbd" => Some(Self::Kbd),
            "details" => Some(Self::Details),
            "summary" => Some(Self::Summary),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::H1 => "h1",
            Self::H2 => "h2",
            Self::H3 => "h3",
            Self::H4 => "h4",
            Self::H5 => "h5",
            Self::H6 => "h6",
            Self::P => "p",
            Self::A => "a",
            Self::Code => "code",
            Self::Pre => "pre",
            Self::Ul => "ul",
            Self::Ol => "ol",
            Self::Li => "li",
            Self::Strong => "strong",
            Self::Em => "em",
            Self::Blockquote => "blockquote",
            Self::Br => "br",
            Self::Img => "img",
            Self::Table => "table",
            Self::Thead => "thead",
            Self::Tbody => "tbody",
            Self::Tr => "tr",
            Self::Th => "th",
            Self::Td => "td",
            Self::Div => "div",
            Self::Span => "span",
            Self::Sup => "sup",
            Self::Kbd => "kbd",
            Self::Details => "details",
            Self::Summary => "summary",
        }
    }

    pub fn is_void(self) -> bool {
        matches!(self, Self::Br | Self::Img)
    }

    pub fn is_skipped_name(name: &str) -> bool {
        matches!(
            name,
            "script" | "style" | "iframe" | "object" | "embed" | "link"
        )
    }
}
