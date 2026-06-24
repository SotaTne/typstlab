use crate::path::HasConfigPath;
use std::path::{Path, PathBuf};

/// f: Path -> Definition(Self)
///
/// ファイルやディレクトリ上の場所から、配置済みの定義を読み込むプロトコル。
/// Config は保存形式にすぎないため、この trait の主語にはしない。
pub trait DefinitionFromPath: Sized + HasConfigPath {
    type Error;

    fn from_path(path: &Path) -> Result<Self, Self::Error>;
}

/// f: Definition(Self) -> Path
///
/// 配置済みの定義を、ファイルやディレクトリ上の場所へ書き出すプロトコル。
pub trait DefinitionToPath: Sized + HasConfigPath {
    type Error;

    fn to_path(&self) -> Result<PathBuf, Self::Error>;
}

/// f: Definition -> Actual(Self)
///
/// 配置済みの定義から、実行・操作できる実体を解決するプロトコル。
pub trait ResolveFromDefinition: Sized {
    type Definition: HasConfigPath;
    type Error;

    fn resolve_from_definition(definition: Self::Definition) -> Result<Self, Self::Error>;
}

/// f: Actual(Self) -> Definition
///
/// 実行・操作できる実体から、配置済みの定義へ戻すプロトコル。
pub trait LockToDefinition: Sized {
    type Definition: HasConfigPath;
    type Error;

    fn lock_to_definition(&self) -> Result<Self::Definition, Self::Error>;
}
