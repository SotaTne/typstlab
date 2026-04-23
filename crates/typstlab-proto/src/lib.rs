use std::path::PathBuf;

pub struct Loaded<Actual, Config> {
    pub actual: Actual,
    pub config: Config,
}

/// 実体（Model）が備えるべき物理法則
pub trait Entity {
    fn path(&self) -> PathBuf;
    fn exists(&self) -> bool {
        self.path().exists()
    }
}

impl<Actual, Config> Entity for Loaded<Actual, Config>
where
    Actual: Entity,
{
    fn path(&self) -> PathBuf {
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

/// 実体がファイルシステムから自身の状態をロード可能であることを示すプロトコル
pub trait Loadable: Entity + Sized {
    type Config;
    type Error: std::error::Error;

    /// 物理ファイルから設定データを読み出す
    fn load_from_disk(&self) -> Result<Self::Config, Self::Error>;

    /// 物理ファイルの状態を自身と設定の組に昇格する
    fn load(self) -> Result<Loaded<Self, Self::Config>, Self::Error> {
        let config = self.load_from_disk()?;
        Ok(Loaded {
            actual: self,
            config,
        })
    }
}

/// アクション（動き）のプロトコル
pub trait Action<Output, Event, Error>
where
    Error: std::error::Error,
{
    fn run(self, monitor: &mut dyn FnMut(Event)) -> Result<Output, Vec<Error>>;
}

pub trait Collection<T, Error>: Entity
where
    T: Entity,
    Error: std::error::Error + 'static,
{
    fn list(&self) -> Result<Vec<T>, Error>;
    fn resolve(&self, input: &str) -> Option<T>;
}

pub trait Artifact: Entity {
    type Error: std::error::Error;

    fn root(&self) -> PathBuf;
    fn is_success(&self) -> bool;
    fn error(&self) -> Option<String>;
    fn files(&self) -> Result<Vec<PathBuf>, Self::Error>;
}

pub trait CliSpeaker<Event, Error, Output> {
    fn render_event(&self, event: Event);
    fn render_error(&self, error: &Error);
    fn render_result(&self, output: &Output);
}

/// AIエージェント向けの Speaker
/// メソッドが String を返すことで、AI へのレスポンスとしてそのまま利用できる
pub trait McpSpeaker<Event, Error, Output> {
    fn render_event(&self, event: Event) -> String;
    fn render_error(&self, error: &Error) -> String;
    fn render_result(&self, output: &Output) -> String;
}

pub trait Validatable {
    type Error: std::error::Error;

    fn validate(&self) -> Result<(), Self::Error>;
}
