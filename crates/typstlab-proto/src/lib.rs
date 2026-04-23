use std::path::PathBuf;

/// 実体（Model）が備えるべき物理法則
pub trait Entity {
    fn path(&self) -> PathBuf;
    fn exists(&self) -> bool {
        self.path().exists()
    }
}

/// 実体が新しく作成可能であることを示すプロトコル
pub trait Creatable: Entity {
    type Args;
    fn initialize(&mut self, args: Self::Args);
    fn persist(&self) -> Result<(), String>;
}

/// 実体がファイルシステムから自身の状態をロード可能であることを示すプロトコル
pub trait Loadable: Entity {
    type Config;
    type Error: std::error::Error;

    /// 物理ファイルから設定データを読み出す
    fn load_from_disk(&self) -> Result<Self::Config, Self::Error>;

    /// 読み出したデータを自分自身の状態に反映する
    fn apply_config(&mut self, config: Self::Config);

    /// 物理ファイルの状態を自分自身に同期する
    fn reload(&mut self) -> Result<(), Self::Error> {
        let config = self.load_from_disk()?;
        self.apply_config(config);
        Ok(())
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
    fn root(&self) -> PathBuf;
    fn is_success(&self) -> bool;
    fn error(&self) -> Option<String>;
    fn files(&self) -> Result<Vec<PathBuf>, String>;
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
    fn validate(&self) -> Result<(), String>;
}
