use crate::{DocsTool, StoredDocsTree};

#[derive(Debug, Clone)]
pub struct ResolvedDocsTree<'a, D>
where
    D: DocsTool,
{
    pub tool: &'a D,
    pub stored: StoredDocsTree,
    pub version: String,
}

impl<'a, D> ResolvedDocsTree<'a, D>
where
    D: DocsTool,
{
    pub fn new(tool: &'a D, stored: StoredDocsTree, version: impl Into<String>) -> Self {
        Self {
            tool,
            stored,
            version: version.into(),
        }
    }
}
