use semver::{Version, VersionReq};

use crate::ToolchainError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolInvocation {
    pub executable: String,
    pub args: Vec<String>,
    pub version_guard: VersionGuard,
}

impl ToolInvocation {
    pub fn new(
        executable: impl Into<String>,
        args: Vec<String>,
        version_guard: VersionGuard,
    ) -> Result<Self, ToolchainError> {
        Ok(Self {
            executable: executable.into(),
            args,
            version_guard,
        })
    }

    pub fn new_all_versions(executable: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            executable: executable.into(),
            args,
            version_guard: VersionGuard::any(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionGuard {
    Any,
    Req(VersionReq),
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
    fn invocation(&self) -> Result<ToolInvocation, ToolchainError>;
}

pub trait RawCommandFactory {
    type Command: ToolCommand;

    fn raw(&self, args: Vec<String>, guard: VersionGuard) -> Self::Command;
}

pub fn assert_raw_command_factory<T: RawCommandFactory>(_factory: &T) {}
