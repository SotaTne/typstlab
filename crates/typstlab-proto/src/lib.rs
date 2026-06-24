pub mod action;
pub mod artifact;
pub mod artifact_result;
pub mod config;
pub mod event;
pub mod install;
pub mod macros;
pub mod models;
pub mod path;
pub mod speaker;
pub mod store;

// 基本定義の再エクスポート
pub use action::Action;
pub use artifact::Artifact;
pub use artifact_result::ArtifactResult;
pub use event::{AppEvent, EventAudience, EventLevel, EventPresentation, EventScope};
pub use install::{Installer, SourceFormat};
pub use models::identity::{Entity, Model};
pub use models::lifecycle::{Creatable, Loadable, Loaded};
pub use models::location::{Locatable, Location, Remote};
pub use speaker::{CliSpeaker, McpSpeaker};
pub use store::{Store, StoreEntryAddress, StoreIndex};

// 定数
pub const PROJECT_SETTING_FILE: &str = "typstlab.toml";
pub const PAPER_SETTING_FILE: &str = "paper.toml";
pub const TEMPLATE_SETTING_FILE: &str = "template.toml";

#[cfg(not(target_os = "windows"))]
pub const TYPST_BINARY_NAME: &str = "typst";

#[cfg(target_os = "windows")]
pub const TYPST_BINARY_NAME: &str = "typst.exe";
