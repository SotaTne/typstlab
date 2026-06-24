/// 配下に意味のあるものがある箱
pub trait HasRoot {
    fn root(&self) -> std::path::PathBuf;
}

/// あるPathをルートとして、そこから意味のあるものへの写像
/// HasRootがあるからと言って、FromRootがあるとは限らない。
/// 設定ファイルを元にした方が信頼性が上がるパターンがあるため
pub trait FromRoot: Sized {
    type Error;

    fn expand_from_root(root: &std::path::Path) -> Result<Self, Self::Error>;
}

/// 配下に意味のあるものがある箱であり、設定ファイルの定義がある
pub trait HasConfigPath: HasRoot {
    fn config_path(&self) -> std::path::PathBuf;
}

/// 配下に同種のものが複数あり構造は問わないが、そのものを特定でき、list的取得ができるもの
pub trait HasEntities: HasRoot {
    type Entity;

    fn entities(&self) -> Vec<Self::Entity>;
    fn find_entity(&self, id: &str) -> Option<Self::Entity>;
}

/// それ自身が意味を持つ個体
pub trait Locatable {
    fn path(&self) -> std::path::PathBuf;
}

/// 任意の入力から、その実体を展開・配置する root path を導くもの
///
/// FromRoot は root から実体を構築するが、ExpandToRoot は入力をもとに
/// 「どこを root として扱うべきか」だけを決定する。
pub trait ExpandToRoot: Sized {
    type Input;
    type Error;

    fn expand_to_root(input: &Self::Input) -> Result<std::path::PathBuf, Self::Error>;
}
