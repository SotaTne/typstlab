use std::collections::HashMap;

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
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct OutlineItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub children: Vec<OutlineItem>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct DocsBody {
    pub kind: String,
    pub content: serde_json::Value,
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
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub example: Option<serde_json::Value>,
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
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ParamContent {
    pub name: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub example: Option<serde_json::Value>,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
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
pub struct TypeContent {
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub oneliner: Option<String>,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub constructor: Option<FuncContent>,
    #[serde(default)]
    pub scope: Vec<FuncContent>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CategoryContent {
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub items: Vec<CategoryItem>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
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
pub struct GroupContent {
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub functions: Vec<FuncContent>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SymbolsContent {
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub list: Vec<SymbolItem>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
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
}
