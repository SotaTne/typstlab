use std::path::PathBuf;

pub const PROJECT_SETTING_FILE: &str = "typstlab.toml";
pub const PAPER_SETTING_FILE: &str = "paper.toml";

pub struct Loaded<Actual, Config> {
    pub actual: Actual,
    pub config: Config,
}

/// Typstlab の世界におけるあらゆる「もの（実体・概念）」の基底プロトコル
pub trait Model {}

/// 実体（Model）が備えるべき物理法則（ローカルに実在するもの）
pub trait Entity: Model {
    fn path(&self) -> PathBuf;
    fn exists(&self) -> bool {
        self.path().exists()
    }
}

impl<Actual, Config> Model for Loaded<Actual, Config> where Actual: Model {}

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
pub trait Action<Output, Event, Warning, Error>
where
    Error: std::error::Error,
{
    fn run(
        self,
        monitor: &mut dyn FnMut(Event),
        warning: &mut dyn FnMut(Warning),
    ) -> Result<Output, Vec<Error>>;
}

/// モデルの集合（ディレクトリ等）を扱うプロトコル
pub trait Collection<T, Error>: Entity
where
    T: Model,
    Error: std::error::Error + 'static,
{
    fn list(&self) -> Result<Vec<T>, Error>;
    fn resolve(&self, input: &str) -> Result<Option<T>, Error>;
}

pub trait Artifact: Entity {
    type Error: std::error::Error;

    fn root(&self) -> PathBuf;
    fn is_success(&self) -> bool;
    fn error(&self) -> Option<String>;
    fn files(&self) -> Result<Vec<PathBuf>, Self::Error>;
}

pub trait CliSpeaker<Event, Warning, Error, Output> {
    fn render_event(&self, event: Event);
    fn render_warning(&self, warning: Warning);
    fn render_error(&self, error: &Error);
    fn render_result(&self, output: &Output);
}

/// AIエージェント向けの Speaker
pub trait McpSpeaker<Event, Warning, Error, Output> {
    fn render_event(&self, event: Event) -> String;
    fn render_warning(&self, warning: Warning) -> String;
    fn render_error(&self, error: &Error) -> String;
    fn render_result(&self, output: &Output) -> String;
}

pub trait Validatable {
    type Error: std::error::Error;

    fn validate(&self) -> Result<(), Self::Error>;
}

/// 実体の所在を表現する
pub enum Location<T> {
    /// ローカルに実在する（物理パスを持つ）
    Local(PathBuf),
    /// 外部に存在する（型 T で定義された識別情報を持つ）
    Remote(T),
}

impl<T> Location<T> {
    /// ローカルであれば PathBuf を返し、リモートであれば None を返す
    pub fn as_local_path(&self) -> Option<&PathBuf> {
        match self {
            Location::Local(path) => Some(path),
            Location::Remote(_) => None,
        }
    }
}

/// 所在を知っていることを示すプロトコル
pub trait Locatable<T>: Model {
    fn location(&self) -> Location<T>;
}

/// 外部リソースを解決するためのプロトコル
pub trait Remote<T, Error>
where
    T: Model,
    Error: std::error::Error + 'static,
{
    /// 識別子から、解決可能なリソースを特定する
    fn resolve_remote(&self, id: &str) -> Result<Option<T>, Error>;
}

#[macro_export]
macro_rules! impl_model {
    ($($t:ty),+) => {
        $(impl $crate::Model for $t {})*
    };
}

#[macro_export]
macro_rules! impl_entity {
    ($t:ty { $($item:tt)* }) => {
        impl $crate::Model for $t {}
        impl $crate::Entity for $t {
            $($item)*
        }
    };
}

#[macro_export]
macro_rules! impl_locatable {
    ($t:ty, $r:ty { $($item:tt)* }) => {
        impl $crate::Model for $t {}
        impl $crate::Locatable<$r> for $t {
            $($item)*
        }
    };
}
