use std::path::PathBuf;
use typstlab_proto::{Creatable, Entity, Loaded};

#[derive(Debug, Clone)]
pub struct Template {
    pub id: String,
    pub path: PathBuf,
}

impl Template {
    pub fn new(id: String, templates_dir: PathBuf) -> Self {
        let path = templates_dir.join(&id);
        Self { id, path }
    }
}

typstlab_proto::impl_entity! {
    Template {
        fn path(&self) -> PathBuf {
            self.path.clone()
        }
    }
}

impl Creatable for Template {
    type Args = ();
    type Config = ();
    type Error = std::io::Error;

    fn initialize(self, _args: Self::Args) -> Result<Loaded<Self, Self::Config>, Self::Error> {
        Ok(Loaded {
            actual: self,
            config: (),
        })
    }

    fn persist(loaded: &Loaded<Self, Self::Config>) -> Result<(), Self::Error> {
        let path = loaded.actual.path();
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }
        let main_typ = path.join("main.typ");
        if !main_typ.exists() {
            std::fs::write(
                main_typ,
                format!(
                    "= Template: {}\n\nTemplate content goes here.",
                    loaded.actual.id
                ),
            )?;
        }
        Ok(())
    }
}
