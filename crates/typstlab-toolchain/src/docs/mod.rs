use crate::{ToolchainError, ToolchainTool};

pub mod typst;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocsSourceFormat {
    RawFile { file_name: &'static str },
    TarXz { strip_components: usize },
    Zip { strip_components: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocsSource {
    pub url: String,
    pub format: DocsSourceFormat,
}

pub trait DocsTool: ToolchainTool {
    fn source(&self, version: &str) -> Result<DocsSource, ToolchainError>;
}
