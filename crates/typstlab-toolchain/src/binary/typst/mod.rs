use std::path::PathBuf;

use crate::binary::{BinaryTool, TypedBinaryTool, VersionCommand, VersionParser};
use crate::{
    InstallLayout, InstallSource, Platform, RawCommandFactory, ToolCommand, ToolInvocation,
    ToolchainError, VersionGuard, VersionResolution,
};

mod platform;

pub static TOOL: Typst = Typst;
pub static COMMAND: TypstCommands = TypstCommands;
pub const TOOL_ID: &str = "typst";

pub struct Typst;
pub struct TypstCommands;

impl BinaryTool for Typst {
    fn id(&self) -> &'static str {
        TOOL_ID
    }

    fn version_resolution(&self) -> VersionResolution {
        VersionResolution::ResolverJson(include_str!("resolver.json"))
    }

    fn version_command(&self) -> Result<VersionCommand, ToolchainError> {
        Ok(VersionCommand {
            invocation: TypstCommand::Version.invocation()?,
            parser: VersionParser::Regex {
                pattern: r"typst\s+([0-9]+\.[0-9]+\.[0-9]+)",
            },
        })
    }

    fn asset_name(&self, target_platform: Platform) -> Result<&'static str, ToolchainError> {
        Ok(platform::resolve(target_platform)?.asset_name)
    }

    fn install_layout(
        &self,
        target_platform: Platform,
    ) -> Result<InstallLayout, ToolchainError> {
        Ok(platform::resolve(target_platform)?.install_layout())
    }

    fn install_source(
        &self,
        version: &str,
        target_platform: Platform,
    ) -> Result<InstallSource, ToolchainError> {
        let target = platform::resolve(target_platform)?;
        Ok(InstallSource {
            url: format!(
                "https://github.com/typst/typst/releases/download/v{version}/{asset_name}.{extension}",
                asset_name = target.asset_name,
                extension = target.archive_extension,
            ),
            layout: self.install_layout(target_platform)?,
        })
    }
}

impl TypedBinaryTool for Typst {
    type Commands = TypstCommands;

    fn commands(&self) -> &'static Self::Commands {
        &COMMAND
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypstCommand {
    Compile {
        source: PathBuf,
        output: Option<PathBuf>,
    },
    Query {
        source: PathBuf,
        selector: String,
    },
    Init {
        template: String,
        output: Option<PathBuf>,
    },
    Update,
    Version,
    Raw {
        args: Vec<String>,
        guard: VersionGuard,
    },
}

impl TypstCommands {
    pub fn compile(&self, source: impl Into<PathBuf>, output: Option<PathBuf>) -> TypstCommand {
        TypstCommand::Compile {
            source: source.into(),
            output,
        }
    }

    pub fn query(&self, source: impl Into<PathBuf>, selector: impl Into<String>) -> TypstCommand {
        TypstCommand::Query {
            source: source.into(),
            selector: selector.into(),
        }
    }

    pub fn init(&self, template: impl Into<String>, output: Option<PathBuf>) -> TypstCommand {
        TypstCommand::Init {
            template: template.into(),
            output,
        }
    }

    pub fn update(&self) -> TypstCommand {
        TypstCommand::Update
    }

    pub fn version(&self) -> TypstCommand {
        TypstCommand::Version
    }
}

impl RawCommandFactory for TypstCommands {
    type Command = TypstCommand;

    fn raw(&self, args: Vec<String>, guard: VersionGuard) -> Self::Command {
        TypstCommand::Raw { args, guard }
    }
}

impl ToolCommand for TypstCommand {
    fn invocation(&self) -> Result<ToolInvocation, ToolchainError> {
        match self {
            Self::Compile { source, output } => {
                let mut args = vec!["compile".to_string(), source.display().to_string()];
                if let Some(output) = output {
                    args.push(output.display().to_string());
                }

                ToolInvocation::new("typst", args, VersionGuard::req(">=0.1.0")?)
            }
            Self::Query { source, selector } => ToolInvocation::new(
                "typst",
                vec![
                    "query".to_string(),
                    source.display().to_string(),
                    selector.clone(),
                ],
                VersionGuard::req(">=0.1.0")?,
            ),
            Self::Init { template, output } => {
                let mut args = vec!["init".to_string(), template.clone()];
                if let Some(output) = output {
                    args.push(output.display().to_string());
                }

                ToolInvocation::new("typst", args, VersionGuard::req(">=0.1.0")?)
            }
            Self::Update => ToolInvocation::new(
                "typst",
                vec!["update".to_string()],
                VersionGuard::req(">=0.1.0")?,
            ),
            Self::Version => Ok(ToolInvocation::new_all_versions(
                "typst",
                vec!["--version".to_string()],
            )),
            Self::Raw { args, guard } => Ok(ToolInvocation {
                executable: "typst".to_string(),
                args: args.clone(),
                version_guard: guard.clone(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Arch, Os, SourceFormat};
    use semver::VersionReq;

    #[test]
    fn compile_command_maps_to_invocation() {
        let invocation = COMMAND
            .compile("main.typ", Some(PathBuf::from("main.pdf")))
            .invocation()
            .unwrap();

        assert_eq!(invocation.executable, "typst");
        assert_eq!(invocation.args, ["compile", "main.typ", "main.pdf"]);
        assert_eq!(
            invocation.version_guard,
            VersionGuard::Req(VersionReq::parse(">=0.1.0").unwrap())
        );
    }

    #[test]
    fn version_command_is_guarded_and_parseable() {
        let command = TOOL.version_command().unwrap();

        assert_eq!(command.invocation.executable, "typst");
        assert_eq!(command.invocation.args, ["--version"]);
        assert_eq!(command.invocation.version_guard, VersionGuard::Any);
    }

    #[test]
    fn command_factory_creates_typed_commands() {
        assert!(matches!(COMMAND.update(), TypstCommand::Update));
        assert!(matches!(COMMAND.version(), TypstCommand::Version));
    }

    #[test]
    fn raw_command_accepts_discontinuous_version_guard() {
        let guard = VersionGuard::any_of(&[">=0.1.0, <0.3.0", ">=0.7.0"]).unwrap();
        let invocation = COMMAND
            .raw(vec!["--help".to_string()], guard.clone())
            .invocation()
            .unwrap();

        assert_eq!(invocation.version_guard, guard);
    }

    #[test]
    fn install_source_resolves_macos_aarch64() {
        let source = TOOL
            .install_source(
                "0.14.2",
                Platform {
                    os: Os::MacOS,
                    arch: Arch::Aarch64,
                },
            )
            .unwrap();

        assert_eq!(
            source.url,
            "https://github.com/typst/typst/releases/download/v0.14.2/typst-aarch64-apple-darwin.tar.xz"
        );
        assert_eq!(
            source.layout.source_format,
            SourceFormat::TarXz {
                strip_components: 1
            }
        );
        assert_eq!(
            source.layout.executable_relative_path,
            PathBuf::from("typst")
        );
    }

    #[test]
    fn install_source_resolves_windows_x86_64() {
        let source = TOOL
            .install_source(
                "0.14.2",
                Platform {
                    os: Os::Windows,
                    arch: Arch::X86_64,
                },
            )
            .unwrap();

        assert_eq!(
            source.url,
            "https://github.com/typst/typst/releases/download/v0.14.2/typst-x86_64-pc-windows-msvc.zip"
        );
        assert_eq!(
            source.layout.source_format,
            SourceFormat::Zip {
                strip_components: 1
            }
        );
        assert_eq!(
            source.layout.executable_relative_path,
            PathBuf::from("typst.exe")
        );
    }

    #[test]
    fn install_source_rejects_unsupported_platform() {
        let error = TOOL
            .install_source(
                "0.14.2",
                Platform {
                    os: Os::MacOS,
                    arch: Arch::Riscv64,
                },
            )
            .unwrap_err();

        assert!(matches!(
            error,
            ToolchainError::UnsupportedPlatform { tool: "typst", .. }
        ));
    }
}
