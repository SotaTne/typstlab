/*
 *
 * typstのバイナリツールの定義とコマンドの実装を提供するモジュールです。
 * ここではTOOLのみが公開されていたら、正しくtypstのバイナリツールを利用できるようになります。
 * 他のバイナリツールでもTOOLを公開することで、同様の方法で利用可能になります。
 *
 */

use std::{ffi::OsString, path::PathBuf};

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
        features: Vec<String>,
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
        args: Vec<OsString>,
        guard: VersionGuard,
    },
}

impl TypstCommands {
    pub fn compile(
        &self,
        source: impl Into<PathBuf>,
        output: Option<PathBuf>,
        features: Vec<String>,
    ) -> TypstCommand {
        TypstCommand::Compile {
            source: source.into(),
            output,
            features,
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

    fn raw(&self, args: Vec<OsString>, guard: VersionGuard) -> Self::Command {
        TypstCommand::Raw { args, guard }
    }
}

impl ToolCommand for TypstCommand {
    fn to_invocation(&self) -> Result<ToolInvocation, ToolchainError> {
        match self {
            Self::Compile {
                source,
                output,
                features,
            } => {
                let mut args = vec!["compile".into(), source.as_os_str().to_owned()];
                if !features.is_empty() {
                    args.extend([OsString::from("--features"), features.join(",").into()]);
                }
                args.extend(output.iter().map(|path| path.as_os_str().to_owned()));

                let minimum = if features
                    .iter()
                    .flat_map(|feature| feature.split(','))
                    .any(|feature| feature == "html")
                {
                    ">=0.13.0"
                } else {
                    ">=0.1.0"
                };
                Ok(ToolInvocation::new(args, VersionGuard::req(minimum)?))
            }
            Self::Query { source, selector } => Ok(ToolInvocation::new(
                vec![
                    "query".into(),
                    source.as_os_str().to_owned(),
                    selector.into(),
                ],
                VersionGuard::req(">=0.5.0")?,
            )),
            Self::Init { template, output } => {
                let mut args = vec!["init".into(), template.into()];
                args.extend(output.iter().map(|path| path.as_os_str().to_owned()));

                Ok(ToolInvocation::new(args, VersionGuard::req(">=0.11.0")?))
            }
            Self::Update => Ok(ToolInvocation::new(
                vec!["update".into()],
                VersionGuard::req(">=0.11.0")?,
            )),
            Self::Version => Ok(ToolInvocation::new_all_versions(vec!["--version".into()])),
            Self::Raw { args, guard } => Ok(ToolInvocation::new(args.clone(), guard.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TypedBinaryTool;
    use crate::binary::BinaryTool;
    use crate::{Arch, BinaryArchiveFormat, Os, Platform};
    use semver::{Version, VersionReq};

    #[test]
    fn compile_command_maps_to_invocation() {
        let invocation = TOOL
            .commands()
            .compile("main.typ", Some(PathBuf::from("main.pdf")), vec![])
            .to_invocation()
            .unwrap();

        assert_eq!(invocation.args, ["compile", "main.typ", "main.pdf"]);
        assert_eq!(
            invocation.version_guard,
            VersionGuard::Req(VersionReq::parse(">=0.1.0").unwrap())
        );
    }

    #[test]
    fn html_feature_raises_compile_version_minimum() {
        let invocation = TOOL
            .commands()
            .compile(
                "main.typ",
                Some(PathBuf::from("output path.html")),
                vec!["html".to_owned()],
            )
            .to_invocation()
            .unwrap();

        assert_eq!(
            invocation.args,
            [
                "compile",
                "main.typ",
                "--features",
                "html",
                "output path.html"
            ]
        );
        assert!(!invocation.version_guard.matches(&Version::new(0, 12, 0)));
        assert!(invocation.version_guard.matches(&Version::new(0, 13, 0)));
        assert!(invocation.version_guard.matches(&Version::new(0, 14, 2)));
    }

    #[test]
    fn typed_commands_set_their_version_boundaries() {
        let commands = TOOL.commands();
        for (command, minimum_minor) in [
            (commands.query("main.typ", "heading"), 5),
            (commands.init("@preview/demo:1.0.0", None), 11),
            (commands.update(), 11),
        ] {
            let guard = command.to_invocation().unwrap().version_guard;
            assert!(!guard.matches(&Version::new(0, minimum_minor - 1, 0)));
            assert!(guard.matches(&Version::new(0, minimum_minor, 0)));
        }
    }

    #[test]
    fn typed_version_matches_the_probe() {
        let command = TOOL.version_command().unwrap();

        assert_eq!(command.invocation.args, ["--version"]);
        assert_eq!(command.invocation.version_guard, VersionGuard::Any);
        assert_eq!(
            command.invocation,
            TOOL.commands().version().to_invocation().unwrap()
        );
    }

    #[test]
    fn command_factory_creates_typed_commands() {
        assert!(matches!(TOOL.commands().update(), TypstCommand::Update));
        assert!(matches!(TOOL.commands().version(), TypstCommand::Version));
    }

    #[test]
    fn invocations_preserve_non_utf8_paths_and_raw_arguments() {
        #[cfg(unix)]
        let argument = {
            use std::os::unix::ffi::OsStringExt;
            OsString::from_vec(b"path with space\xff".to_vec())
        };
        #[cfg(windows)]
        let argument = {
            use std::os::windows::ffi::OsStringExt;
            OsString::from_wide(&[b'p' as u16, 0xd800])
        };
        assert!(argument.to_str().is_none());

        let commands = TOOL.commands();
        let path = PathBuf::from(&argument);
        let compile = commands
            .compile(&path, Some(path.clone()), vec![])
            .to_invocation()
            .unwrap();
        assert_eq!(compile.args[1..], [argument.clone(), argument.clone()]);
        let query = commands.query(&path, "heading").to_invocation().unwrap();
        assert_eq!(query.args[1], argument);
        let init = commands
            .init("@preview/demo:1.0.0", Some(path))
            .to_invocation()
            .unwrap();
        assert_eq!(init.args[2], argument);

        let guard = VersionGuard::any_of(&[">=0.1.0, <0.3.0", ">=0.7.0"]).unwrap();
        let invocation = commands
            .raw(vec![argument.clone()], guard.clone())
            .to_invocation()
            .unwrap();

        assert_eq!(invocation.args, [argument]);
        assert_eq!(invocation.version_guard, guard);
    }

    #[test]
    fn typst_has_an_asset_for_every_supported_target_triple() {
        for os in [Os::MacOS, Os::Linux, Os::Windows] {
            for arch in [Arch::X86_64, Arch::Aarch64, Arch::Riscv64] {
                let platform = Platform { os, arch };
                if let Ok(triple) = platform.target_triple() {
                    assert_eq!(
                        TOOL.asset_name(platform).unwrap(),
                        format!("typst-{triple}")
                    );
                }
            }
        }
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
