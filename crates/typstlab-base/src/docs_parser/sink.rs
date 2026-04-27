use std::path::Path;

use super::{DocsRenderError, route};

pub trait DocsRenderSink {
    fn write_markdown(
        &mut self,
        relative_path: &Path,
        content: &str,
    ) -> Result<(), DocsRenderError>;
}

#[derive(Debug)]
pub struct RenderedDocs {
    tempdir: tempfile::TempDir,
    file_count: usize,
}

impl RenderedDocs {
    pub fn path(&self) -> &Path {
        self.tempdir.path()
    }

    pub fn file_count(&self) -> usize {
        self.file_count
    }
}

impl AsRef<Path> for RenderedDocs {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

pub struct TempDocsRenderSink {
    tempdir: tempfile::TempDir,
    file_count: usize,
}

impl TempDocsRenderSink {
    pub fn new() -> Result<Self, DocsRenderError> {
        let tempdir = tempfile::TempDir::new().map_err(DocsRenderError::TempDir)?;
        Ok(Self {
            tempdir,
            file_count: 0,
        })
    }

    pub fn into_rendered_docs(self) -> RenderedDocs {
        RenderedDocs {
            tempdir: self.tempdir,
            file_count: self.file_count,
        }
    }
}

impl DocsRenderSink for TempDocsRenderSink {
    fn write_markdown(
        &mut self,
        relative_path: &Path,
        content: &str,
    ) -> Result<(), DocsRenderError> {
        route::validate_output_path(relative_path)?;

        let output_path = self.tempdir.path().join(relative_path);
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(output_path, content)?;
        self.file_count += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn test_temp_docs_render_sink_rejects_escape_path() {
        let mut sink = TempDocsRenderSink::new().unwrap();

        let err = sink
            .write_markdown(Path::new("../escape.md"), "escape")
            .unwrap_err();

        assert!(matches!(err, DocsRenderError::PathTraversal(_)));
    }
}
