use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Typst {
    pub version: String,
    pub binary_path: PathBuf,
}

typstlab_proto::impl_entity! {
    Typst {
        fn path(&self) -> PathBuf {
            self.binary_path.clone()
        }
    }
}

impl Typst {
    pub fn new(version: String, binary_path: PathBuf) -> Self {
        Self {
            version,
            binary_path,
        }
    }
}
