use crate::docs::{DocsSource, DocsSourceFormat, DocsTool};
use crate::{ToolchainError, ToolchainTool, VersionResolution};

pub const RAW_DOCS_FILENAME: &str = "downloaded.raw";

pub static TOOL: TypstDocs = TypstDocs;

pub struct TypstDocs;

impl ToolchainTool for TypstDocs {
    fn id(&self) -> &'static str {
        "typst-docs"
    }

    fn version_resolution(&self) -> VersionResolution {
        VersionResolution::ResolverJson(include_str!("resolver.json"))
    }
}

impl DocsTool for TypstDocs {
    fn source(&self, version: &str) -> Result<DocsSource, ToolchainError> {
        Ok(DocsSource {
            url: format!(
                "https://github.com/typst-community/dev-builds/releases/download/docs-v{version}/docs.json"
            ),
            format: DocsSourceFormat::RawFile {
                file_name: RAW_DOCS_FILENAME,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_uses_typst_docs_dev_builds_release() {
        let source = TOOL.source("0.14.2").unwrap();

        assert_eq!(
            source.url,
            "https://github.com/typst-community/dev-builds/releases/download/docs-v0.14.2/docs.json"
        );
        assert_eq!(
            source.format,
            DocsSourceFormat::RawFile {
                file_name: RAW_DOCS_FILENAME
            }
        );
    }
}
