use std::path::PathBuf;
use typstlab_proto::Entity;

pub struct Docs {
    pub project_root: PathBuf,
    pub version: String,
}

impl Docs {
    pub fn new(project_root: PathBuf, version: String) -> Self {
        Self { project_root, version }
    }
}

impl Entity for Docs {
    fn path(&self) -> PathBuf {
        self.project_root.join(".typstlab/kb/typst/docs")
    }
}
