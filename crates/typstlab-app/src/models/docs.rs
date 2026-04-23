use std::path::PathBuf;

pub struct Docs {
    pub path: PathBuf,
}

typstlab_proto::impl_entity! {
    Docs {
        fn path(&self) -> PathBuf {
            self.path.clone()
        }
    }
}

impl Docs {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}
