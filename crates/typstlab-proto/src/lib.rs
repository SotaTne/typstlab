pub mod action;
pub mod install;
pub mod macros;
pub mod models;
pub mod speaker;

// 基本定義の再エクスポート
pub use action::Action;
pub use install::{Installer, SourceFormat};
pub use models::artifact::Artifact;
pub use models::identity::{Entity, Model};
pub use models::lifecycle::{Creatable, Loadable, Loaded};
pub use models::location::{Locatable, Location, Remote};
pub use models::store::{Collection, Store};
pub use speaker::{CliSpeaker, McpSpeaker};

// 定数
pub const PROJECT_SETTING_FILE: &str = "typstlab.toml";
pub const PAPER_SETTING_FILE: &str = "paper.toml";
