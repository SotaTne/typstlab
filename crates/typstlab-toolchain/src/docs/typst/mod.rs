use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::docs::{DocsSource, DocsSourceFormat, DocsTool, RenderedDocs, raw_file_path};
use crate::{ToolchainError, VersionResolution};

pub mod parser;

pub const RAW_DOCS_FILENAME: &str = "downloaded.raw";

pub static TOOL: TypstDocs = TypstDocs;

pub struct TypstDocs;

impl DocsTool for TypstDocs {
    fn id(&self) -> &'static str {
        "typst-docs"
    }

    fn version_resolution(&self) -> VersionResolution {
        VersionResolution::ResolverJson(include_str!("resolver.json"))
    }

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

    fn render(&self, source_root: &Path) -> Result<RenderedDocs, ToolchainError> {
        let raw_path = raw_file_path(source_root, RAW_DOCS_FILENAME);
        let raw = File::open(raw_path)?;
        parser::render_docs_from_reader(BufReader::new(raw)).map_err(|error| {
            ToolchainError::DocsTransform {
                tool: self.id(),
                message: error.to_string(),
            }
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

    #[test]
    fn render_converts_docs_json_to_tempdir_backed_markdown_tree() {
        let source = tempfile::TempDir::new().unwrap();
        std::fs::write(
            source.path().join(RAW_DOCS_FILENAME),
            r#"[
                {
                    "route": "/DOCS-BASE/",
                    "title": "Overview",
                    "body": { "kind": "html", "content": "<p>Hello docs</p>" },
                    "children": []
                }
            ]"#,
        )
        .unwrap();

        let rendered = TOOL.render(source.path()).unwrap();

        let markdown = std::fs::read_to_string(rendered.path().join("index.md")).unwrap();
        assert_eq!(rendered.file_count(), 1);
        assert!(markdown.contains("title: Overview"));
        assert!(markdown.contains("Hello docs"));
    }
}
