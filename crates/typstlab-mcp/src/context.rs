use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct McpContext {
    pub project_root: PathBuf,
}

impl McpContext {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }
}
