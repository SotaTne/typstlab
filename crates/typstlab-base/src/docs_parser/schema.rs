use super::html::Html;
use serde::{Deserialize, Serialize};

/// Parsed docs.json node.
///
/// This is a parser-level representation, not an application domain model.
/// Do not store it as final docs state.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct DocsEntry {
    pub route: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub part: Option<String>,
    #[serde(default)]
    pub outline: Vec<OutlineItem>,
    #[serde(default)]
    pub body: Option<DocsBody>,
    #[serde(default)]
    pub children: Vec<DocsEntry>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct OutlineItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub children: Vec<OutlineItem>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", content = "content")]
pub enum DocsBody {
    #[serde(rename = "html")]
    Html(Html),
    #[serde(rename = "func")]
    Func(FuncContent),
    #[serde(rename = "type")]
    Type(TypeContent),
    #[serde(rename = "category")]
    Category(CategoryContent),
    #[serde(rename = "group")]
    Group(GroupContent),
    #[serde(rename = "symbols")]
    Symbols(SymbolsContent),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FuncContent {
    #[serde(default)]
    pub path: Vec<String>,
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub oneliner: Option<String>,
    #[serde(default)]
    pub details: Option<RichContent>,
    #[serde(default)]
    pub example: Option<String>,
    #[serde(default)]
    pub params: Vec<ParamContent>,
    #[serde(default)]
    pub returns: Vec<String>,
    #[serde(default)]
    pub scope: Vec<FuncContent>,
    #[serde(default)]
    pub element: bool,
    #[serde(default)]
    pub contextual: bool,
    #[serde(default, rename = "self")]
    pub self_param: bool,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub deprecation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ParamContent {
    pub name: String,
    #[serde(default)]
    pub details: Option<RichContent>,
    #[serde(default)]
    pub example: Option<String>,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub strings: Vec<StringChoice>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub positional: bool,
    #[serde(default)]
    pub named: bool,
    #[serde(default)]
    pub variadic: bool,
    #[serde(default)]
    pub settable: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StringChoice {
    pub string: String,
    #[serde(default)]
    pub details: Option<RichContent>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TypeContent {
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub oneliner: Option<String>,
    #[serde(default)]
    pub details: Option<RichContent>,
    #[serde(default)]
    pub constructor: Option<FuncContent>,
    #[serde(default)]
    pub scope: Vec<FuncContent>,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CategoryContent {
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub details: Option<RichContent>,
    #[serde(default)]
    pub items: Vec<CategoryItem>,
    #[serde(default)]
    pub shorthands: Option<ShorthandsContent>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CategoryItem {
    pub name: String,
    pub route: String,
    #[serde(default)]
    pub oneliner: Option<String>,
    #[serde(default)]
    pub code: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ShorthandsContent {
    #[serde(default)]
    pub markup: Vec<SymbolItem>,
    #[serde(default)]
    pub math: Vec<SymbolItem>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct GroupContent {
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub details: Option<RichContent>,
    #[serde(default)]
    pub functions: Vec<FuncContent>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SymbolsContent {
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub details: Option<RichContent>,
    #[serde(default)]
    pub list: Vec<SymbolItem>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SymbolItem {
    pub name: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub codepoint: Option<u32>,
    #[serde(default, rename = "markupShorthand")]
    pub markup_shorthand: Option<String>,
    #[serde(default, rename = "mathShorthand")]
    pub math_shorthand: Option<String>,
    #[serde(default)]
    pub accent: bool,
    #[serde(default)]
    pub alternates: Vec<String>,
    #[serde(default, rename = "mathClass")]
    pub math_class: Option<String>,
    #[serde(default)]
    pub deprecation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RichContent {
    Plain(String),
    Blocks(Vec<RichBlock>),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", content = "content")]
pub enum RichBlock {
    #[serde(rename = "html")]
    Html(Html),
    #[serde(rename = "example")]
    Example(ExampleContent),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ExampleContent {
    pub body: Html,
    #[serde(default)]
    pub title: Option<String>,
}
