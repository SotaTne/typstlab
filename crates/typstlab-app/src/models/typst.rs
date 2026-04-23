use std::path::PathBuf;
use typstlab_proto::Entity;

pub struct Typst {
    pub version: String,
    pub binary_path: PathBuf,
}

impl Entity for Typst {
    fn path(&self) -> PathBuf {
        self.binary_path.clone()
    }
}
