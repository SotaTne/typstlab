use std::path::PathBuf;

/// 実体（Model）が備えるべき物理法則
pub trait Entity {
    fn path(&self) -> PathBuf;
    fn exists(&self) -> bool {
        self.path().exists()
    }
}

/// アクション（動き）のプロトコル
pub trait Action<Output, Event, Error>
where
    Error: std::error::Error,
{
    fn run(&self, monitor: &mut dyn FnMut(Event)) -> Result<Output, Vec<Error>>;
}

/// 領土（Collection/Scope）プロトコル
pub trait Collection<T, Error>: Entity
where
    T: Entity,
    Error: std::error::Error + 'static,
{
    fn list(&self) -> Result<Vec<T>, Error>;
    fn resolve(&self, input: &str) -> Option<T>;
}

/// 成果物（Artifact）プロトコル
pub trait Artifact: Entity {
    fn root(&self) -> PathBuf;
    fn is_success(&self) -> bool;
    fn error(&self) -> Option<String>;
    fn files(&self) -> Result<Vec<PathBuf>, String>;
}

/// 人間（CLI）との対話プロトコル
pub trait CliSpeaker<Event, Error, Output> {
    /// 進行中のイベントを語る
    fn render_event(&self, event: Event);
    /// 失敗（エラー）を語る
    fn render_error(&self, error: &Error);
    /// 最終的な成果（成功）を語る
    fn render_result(&self, output: &Output);
}

/// AIエージェント（MCP）との対話プロトコル
pub trait McpSpeaker<Event, Error, Output> {
    fn render_event(&self, event: Event) -> String;
    fn render_error(&self, error: &Error) -> String;
    fn render_result(&self, output: &Output) -> String;
}

pub trait Validatable {
    fn validate(&self) -> Result<(), String>;
}
