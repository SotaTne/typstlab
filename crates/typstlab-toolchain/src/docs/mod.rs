use std::path::{Path, PathBuf};

use crate::{ToolchainError, VersionResolution};

pub mod typst;

pub static DOCS: &[&dyn DocsTool] = &[&typst::TOOL];

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

#[derive(Debug)]
pub struct RenderedDocs {
    tempdir: tempfile::TempDir,
    file_count: usize,
}

impl RenderedDocs {
    pub(crate) fn from_tempdir(tempdir: tempfile::TempDir, file_count: usize) -> Self {
        Self {
            tempdir,
            file_count,
        }
    }

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

pub trait DocsTool: Sync {
    fn id(&self) -> &'static str;
    fn version_resolution(&self) -> VersionResolution;
    fn source(&self, version: &str) -> Result<DocsSource, ToolchainError>;
    fn render(&self, source_root: &Path) -> Result<RenderedDocs, ToolchainError>;
}

pub(crate) fn raw_file_path(source_root: &Path, file_name: &str) -> PathBuf {
    source_root.join(file_name)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::{CompatibilityTable, VersionResolution};

    #[test]
    fn every_docs_tool_has_unique_id() {
        let mut ids = BTreeSet::new();

        for tool in DOCS {
            assert!(
                ids.insert(tool.id()),
                "duplicate docs tool id: {}",
                tool.id()
            );
        }
    }

    #[test]
    fn every_docs_tool_has_valid_version_resolution() {
        for tool in DOCS {
            match tool.version_resolution() {
                VersionResolution::ResolverJson(json) => {
                    CompatibilityTable::from_json_str(tool.id(), json).unwrap();
                }
                VersionResolution::TypstlabVersion => {}
            }
        }
    }
}
