/*
 *
 * typstのバイナリツールの定義とコマンドの実装を提供するモジュールです。
 * ここではTOOLのみが公開されていたら、正しくtypstのバイナリツールを利用できるようになります。
 * 他のバイナリツールでもTOOLを公開することで、同様の方法で利用可能になります。
 *
 */

use std::path::PathBuf;

use crate::binary::{GithubBinaryTool, GithubReleaseSource, VersionParser, VersionProbe};
use crate::{
    RawCommandFactory, ToolCommand, ToolInvocation, ToolchainError, VersionGuard, VersionResolution,
};

mod platform;

static COMMANDS: TypstCommands = TypstCommands;

const TOOL_ID: &str = "typst";

const GITHUB_SOURCE: GithubReleaseSource = GithubReleaseSource {
    owner: "typst",
    repo: "typst",
    tag_prefix: "v",
};

pub static TOOL: GithubBinaryTool<TypstCommands> = GithubBinaryTool {
    id: TOOL_ID,
    commands: &COMMANDS,
    github: GITHUB_SOURCE,
    version_resolution: VersionResolution::ResolverJson(include_str!("resolver.json")),
    version_probe: VersionProbe {
        args: &["--version"],
        parser: VersionParser::Regex {
            pattern: r"typst\s+([0-9]+\.[0-9]+\.[0-9]+)",
        },
    },
    platforms: platform::SPECS,
};

pub struct TypstCommands;

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
    fn to_invocation(&self) -> Result<ToolInvocation, ToolchainError> {
        match self {
            Self::Compile { source, output } => {
                let mut args = vec!["compile".to_string(), source.display().to_string()];
                if let Some(output) = output {
                    args.push(output.display().to_string());
                }

                Ok(ToolInvocation::new(args, VersionGuard::req(">=0.1.0")?))
            }
            Self::Query { source, selector } => Ok(ToolInvocation::new(
                vec![
                    "query".to_string(),
                    source.display().to_string(),
                    selector.clone(),
                ],
                VersionGuard::req(">=0.1.0")?,
            )),
            Self::Init { template, output } => {
                let mut args = vec!["init".to_string(), template.clone()];
                if let Some(output) = output {
                    args.push(output.display().to_string());
                }

                Ok(ToolInvocation::new(args, VersionGuard::req(">=0.1.0")?))
            }
            Self::Update => Ok(ToolInvocation::new(
                vec!["update".to_string()],
                VersionGuard::req(">=0.1.0")?,
            )),
            Self::Version => Ok(ToolInvocation::new_all_versions(vec![
                "--version".to_string(),
            ])),
            Self::Raw { args, guard } => Ok(ToolInvocation {
                args: args.clone(),
                version_guard: guard.clone(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TypedBinaryTool;
    use crate::binary::BinaryTool;
    use crate::{Arch, BinaryArchiveFormat, Os, Platform};
    use semver::VersionReq;

    #[test]
    fn compile_command_maps_to_invocation() {
        let invocation = TOOL
            .commands()
            .compile("main.typ", Some(PathBuf::from("main.pdf")))
            .to_invocation()
            .unwrap();

        assert_eq!(invocation.args, ["compile", "main.typ", "main.pdf"]);
        assert_eq!(
            invocation.version_guard,
            VersionGuard::Req(VersionReq::parse(">=0.1.0").unwrap())
        );
    }

    #[test]
    fn version_command_is_guarded_and_parseable() {
        let command = TOOL.version_command().unwrap();

        assert_eq!(command.invocation.args, ["--version"]);
        assert_eq!(command.invocation.version_guard, VersionGuard::Any);
    }

    #[test]
    fn command_factory_creates_typed_commands() {
        assert!(matches!(TOOL.commands().update(), TypstCommand::Update));
        assert!(matches!(TOOL.commands().version(), TypstCommand::Version));
    }

    #[test]
    fn raw_command_accepts_discontinuous_version_guard() {
        let guard = VersionGuard::any_of(&[">=0.1.0, <0.3.0", ">=0.7.0"]).unwrap();
        let invocation = TOOL
            .commands()
            .raw(vec!["--help".to_string()], guard.clone())
            .to_invocation()
            .unwrap();

        assert_eq!(invocation.version_guard, guard);
    }

    #[test]
    fn distribution_resolves_macos_aarch64() {
        let source = TOOL
            .distribution(
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
            source.layout.archive_format,
            BinaryArchiveFormat::TarXz {
                strip_components: 1
            }
        );
        assert_eq!(
            source.layout.executable_relative_path,
            PathBuf::from("typst")
        );
    }

    #[test]
    fn distribution_resolves_windows_x86_64() {
        let source = TOOL
            .distribution(
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
            source.layout.archive_format,
            BinaryArchiveFormat::Zip {
                strip_components: 1
            }
        );
        assert_eq!(
            source.layout.executable_relative_path,
            PathBuf::from("typst.exe")
        );
    }

    #[test]
    fn distribution_rejects_unsupported_platform() {
        let error = TOOL
            .distribution(
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
