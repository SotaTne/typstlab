use super::render;
use serde::Serialize;
use std::fmt;

pub trait ToMarkdown {
    fn to_markdown(&self) -> String;
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Document {
    pub children: Vec<Block>,
}

impl Document {
    pub fn new(children: Vec<Block>) -> Self {
        Self { children }
    }

    pub fn push(&mut self, block: Block) {
        self.children.push(block);
    }
}

impl ToMarkdown for Document {
    fn to_markdown(&self) -> String {
        render::document_to_markdown(self)
    }
}

impl fmt::Display for Document {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_markdown())
    }
}

impl From<Document> for String {
    fn from(document: Document) -> Self {
        document.to_markdown()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Block {
    Paragraph(Vec<Inline>),
    Heading { depth: u8, children: Vec<Inline> },
    Code { lang: Option<String>, value: String },
    Blockquote(Vec<Block>),
    List { ordered: bool, items: Vec<ListItem> },
    Table(Table),
    ThematicBreak,
}

impl ToMarkdown for Block {
    fn to_markdown(&self) -> String {
        render::block_to_markdown(self)
    }
}

impl fmt::Display for Block {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_markdown())
    }
}

impl From<Block> for String {
    fn from(block: Block) -> Self {
        block.to_markdown()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ListItem {
    pub children: Vec<Block>,
}

impl ListItem {
    pub fn new(children: Vec<Block>) -> Self {
        Self { children }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Table {
    pub rows: Vec<TableRow>,
    pub align: Vec<TableAlign>,
}

impl Table {
    pub fn new(rows: Vec<TableRow>) -> Self {
        Self {
            rows,
            align: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub enum TableAlign {
    #[default]
    None,
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

impl TableRow {
    pub fn new(cells: Vec<TableCell>) -> Self {
        Self { cells }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TableCell {
    pub children: Vec<Inline>,
}

impl TableCell {
    pub fn new(children: Vec<Inline>) -> Self {
        Self { children }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Inline {
    Text(String),
    Code(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Link {
        children: Vec<Inline>,
        url: String,
        title: Option<String>,
    },
    Image {
        alt: String,
        url: String,
        title: Option<String>,
    },
    Break,
}

impl Inline {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    pub fn code(value: impl Into<String>) -> Self {
        Self::Code(value.into())
    }
}

impl ToMarkdown for Inline {
    fn to_markdown(&self) -> String {
        render::inline_to_markdown(self)
    }
}

impl fmt::Display for Inline {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_markdown())
    }
}

impl From<Inline> for String {
    fn from(inline: Inline) -> Self {
        inline.to_markdown()
    }
}
