use anyhow::{Result, anyhow};
use semver::{Version, VersionReq};
use std::path::PathBuf;
use std::process::Command;

/// Typst の主要なコマンドを型定義
pub enum TypstCommand {
    Compile {
        source: PathBuf,
        output: Option<PathBuf>,
    },
    Query {
        source: PathBuf,
        selector: String,
    },
    Update,
    Version,
    /// 生の引数を直接渡す実行（バージョンガード付き）
    Raw {
        args: Vec<String>,
        require: VersionReq,
    },
}

impl TypstCommand {
    /// そのコマンドを実行するために必要な最低限のバージョン条件
    pub fn required_version(&self) -> VersionReq {
        match self {
            TypstCommand::Compile { .. } => VersionReq::parse(">=0.1.0").unwrap(),
            TypstCommand::Query { .. } => VersionReq::parse(">=0.5.0").unwrap(),
            TypstCommand::Update => VersionReq::parse(">=0.11.0").unwrap(),
            TypstCommand::Version => VersionReq::parse("*").unwrap(),
            TypstCommand::Raw { require, .. } => require.clone(),
        }
    }

    pub fn to_args(&self) -> Vec<String> {
        match self {
            TypstCommand::Compile { source, output } => {
                let mut args = vec!["compile".to_string(), source.to_string_lossy().to_string()];
                if let Some(out) = output {
                    args.push(out.to_string_lossy().to_string());
                }
                args
            }
            TypstCommand::Query { source, selector } => {
                vec![
                    "query".to_string(),
                    source.to_string_lossy().to_string(),
                    selector.clone(),
                ]
            }
            TypstCommand::Update => vec!["update".to_string()],
            TypstCommand::Version => vec!["--version".to_string()],
            TypstCommand::Raw { args, .. } => args.clone(),
        }
    }
}

pub struct TypstDriver {
    pub binary_path: PathBuf,
}

pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

impl TypstDriver {
    pub fn new(binary_path: PathBuf) -> Self {
        Self { binary_path }
    }

    /// 現在のバイナリのバージョンを取得 (再帰を避けるため raw 実行を使用)
    pub fn get_version(&self) -> Result<Version> {
        let res = self.execute_raw(TypstCommand::Version.to_args())?;

        let v_str = res
            .stdout
            .split_whitespace()
            .nth(1)
            .ok_or_else(|| anyhow!("Failed to parse typst version output"))?;

        Version::parse(v_str).map_err(|e| anyhow!("Invalid semver: {}", e))
    }

    /// 型安全なコマンドを受け取って、バージョンガードを通した上で実行
    pub fn execute(&self, command: TypstCommand) -> Result<ExecutionResult> {
        // 1. バージョンガード (Version コマンド自体の場合はガードをスキップして無限再帰を防ぐ)
        if !matches!(command, TypstCommand::Version) {
            let current_v = self.get_version()?;
            let required_req = command.required_version();

            if !required_req.matches(&current_v) {
                return Err(anyhow!(
                    "Typst version guard failed: current version {} does not match requirement {}",
                    current_v,
                    required_req
                ));
            }
        }

        // 2. 実行
        self.execute_raw(command.to_args())
    }

    fn execute_raw(&self, args: Vec<String>) -> Result<ExecutionResult> {
        use std::time::Instant;
        let start = Instant::now();

        let output = Command::new(&self.binary_path).args(args).output()?;
        let duration = start.elapsed().as_millis() as u64;

        Ok(ExecutionResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration_ms: duration,
        })
    }
}
