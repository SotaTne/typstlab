use crate::{ToolInvocation, VersionGuard};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionParser {
    /// version command の stdout/stderr から、最初の capture group を version として読む。
    Regex { pattern: &'static str },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionCommand {
    /// version を読むために実行する command。
    pub invocation: ToolInvocation,
    /// command output から version を抽出する parser。
    pub parser: VersionParser,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionProbe {
    /// tool 側に定義する静的な version command 引数。
    pub args: &'static [&'static str],
    /// version command output の解釈方法。
    pub parser: VersionParser,
}

impl VersionProbe {
    pub fn to_command(&self) -> VersionCommand {
        VersionCommand {
            invocation: ToolInvocation::new(
                self.args.iter().map(|arg| (*arg).to_string()).collect(),
                VersionGuard::Any,
            ),
            parser: self.parser.clone(),
        }
    }
}
