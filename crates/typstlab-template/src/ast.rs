#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateAst {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub kind: NodeKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Raw(String),
    Display(ParamRef),
    Literal(ParamRef),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Half-open byte range in the original source text.
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParamRef {
    pub name: String,
}

impl Node {
    pub fn new(kind: NodeKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

impl ParamRef {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}
