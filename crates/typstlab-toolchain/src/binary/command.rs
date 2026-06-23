use semver::{Version, VersionReq};

use crate::ToolchainError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolInvocation {
    /// executable は runtime 側で解決するため、ここでは引数列だけを持つ。
    pub args: Vec<String>,
    /// この invocation が実行可能な tool version の条件。
    pub version_guard: VersionGuard,
}

impl ToolInvocation {
    pub fn new(args: Vec<String>, version_guard: VersionGuard) -> Self {
        Self {
            args,
            version_guard,
        }
    }

    pub fn new_all_versions(args: Vec<String>) -> Self {
        Self {
            args,
            version_guard: VersionGuard::any(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionGuard {
    /// 全 version で使える command。
    Any,
    /// 単一の semver requirement を満たす version だけで使える command。
    Req(VersionReq),
    /// 複数の requirement のどれかを満たせば使える command。
    AnyOf(Vec<VersionReq>),
}

impl VersionGuard {
    pub fn any() -> Self {
        Self::Any
    }

    pub fn req(requirement: &str) -> Result<Self, ToolchainError> {
        Ok(Self::Req(VersionReq::parse(requirement)?))
    }

    pub fn any_of(requirements: &[&str]) -> Result<Self, ToolchainError> {
        let requirements = requirements
            .iter()
            .map(|requirement| VersionReq::parse(requirement))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self::AnyOf(requirements))
    }

    pub fn matches(&self, version: &Version) -> bool {
        match self {
            Self::Any => true,
            Self::Req(requirement) => requirement.matches(version),
            Self::AnyOf(requirements) => requirements
                .iter()
                .any(|requirement| requirement.matches(version)),
        }
    }
}

pub trait ToolCommand {
    /// 型付き command を、実行前の中間表現へ変換する。
    /// ここではまだ executable path を知らない。
    fn to_invocation(&self) -> Result<ToolInvocation, ToolchainError>;
}

pub trait RawCommandFactory {
    type Command: ToolCommand;

    /// 型付き API にない command を使うための escape hatch。
    /// raw でも version guard を必須にして、無条件実行を避ける。
    fn raw(&self, args: Vec<String>, guard: VersionGuard) -> Self::Command;
}
