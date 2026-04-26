use crate::models::identity::{Entity, Model};

/// 実体とその設定データをペアで保持する状態
pub struct Loaded<Actual, Config> {
    pub actual: Actual,
    pub config: Config,
}

impl<Actual, Config> Model for Loaded<Actual, Config> where Actual: Model {}

impl<Actual, Config> Entity for Loaded<Actual, Config>
where
    Actual: Entity,
{
    fn path(&self) -> std::path::PathBuf {
        self.actual.path()
    }
}

/// 実体が新しく作成可能であることを示すプロトコル
pub trait Creatable: Entity + Sized {
    type Args;
    type Config;
    type Error: std::error::Error;

    fn initialize(self, args: Self::Args) -> Result<Loaded<Self, Self::Config>, Self::Error>;
    fn persist(loaded: &Loaded<Self, Self::Config>) -> Result<(), Self::Error>;
}

/// 実体がロード可能であることを示すプロトコル
pub trait Loadable: Entity + Sized {
    type Config;
    type Error: std::error::Error;

    fn load_from_disk(&self) -> Result<Self::Config, Self::Error>;

    fn load(self) -> Result<Loaded<Self, Self::Config>, Self::Error> {
        let config = self.load_from_disk()?;
        Ok(Loaded {
            actual: self,
            config,
        })
    }
}
