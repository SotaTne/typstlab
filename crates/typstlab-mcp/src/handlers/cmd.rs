use crate::errors;
use crate::handlers::{Safety, ToolExt};
use crate::server::TypstlabServer;
use chrono::Utc;
use futures_util::FutureExt;
use rmcp::{
    ErrorData as McpError,
    handler::server::common::FromContextPart,
    handler::server::router::tool::{ToolRoute, ToolRouter},
    handler::server::wrapper::Parameters,
    model::*,
    schemars, serde,
};
use std::borrow::Cow;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use tokio::sync::Semaphore;
use tokio::task::spawn_blocking;
use typstlab_core::paper::Paper;
use typstlab_core::path::is_safe_single_component;
use typstlab_core::project::{Project, generate_paper};
use typstlab_core::state::{BuildState, LastBuild, State};
use typstlab_core::status::StatusEngine;
use typstlab_typst::exec::{ExecOptions, ExecResult, exec_typst};

pub struct CmdTool;

impl CmdTool {
    // For integration tests
    pub async fn test_cmd_build(
        server: &TypstlabServer,
        args: BuildArgs,
    ) -> Result<CallToolResult, McpError> {
        Self::build(server, args).await
    }

    pub async fn test_cmd_generate(
        server: &TypstlabServer,
        args: CmdGenerateArgs,
    ) -> Result<CallToolResult, McpError> {
        Self::cmd_generate(server, args).await
    }

    pub async fn test_status(
        server: &TypstlabServer,
        args: StatusArgs,
    ) -> Result<CallToolResult, McpError> {
        Self::status(server, args).await
    }

    pub fn into_router(self) -> ToolRouter<TypstlabServer> {
        ToolRouter::new()
            .with_route(ToolRoute::new_dyn(Self::cmd_generate_attr(), |mut ctx| {
                let server = ctx.service;
                let args_res = Parameters::<CmdGenerateArgs>::from_context_part(&mut ctx);
                async move {
                    let Parameters(args) = args_res?;
                    Self::cmd_generate(server, args).await
                }
                .boxed()
            }))
            .with_route(ToolRoute::new_dyn(Self::status_attr(), |mut ctx| {
                let server = ctx.service;
                let args_res = Parameters::<StatusArgs>::from_context_part(&mut ctx);
                async move {
                    let Parameters(args) = args_res?;
                    Self::status(server, args).await
                }
                .boxed()
            }))
            .with_route(ToolRoute::new_dyn(Self::build_attr(), |mut ctx| {
                let server = ctx.service;
                let args_res = Parameters::<BuildArgs>::from_context_part(&mut ctx);
                async move {
                    let Parameters(args) = args_res?;
                    Self::build(server, args).await
                }
                .boxed()
            }))
            .with_route(ToolRoute::new_dyn(
                Self::typst_docs_status_attr(),
                |mut _ctx| {
                    let server = _ctx.service;
                    async move { Self::typst_docs_status(server).await }.boxed()
                },
            ))
    }

    /// Create a router with only offline-safe tools (network: false)
    ///
    /// This router includes only tools that do not require network access:
    /// - cmd_status: Read-only status checks
    /// - cmd_typst_docs_status: Check Typst documentation status
    ///
    /// Network-dependent tools (cmd_generate, cmd_build) are excluded.
    pub fn into_router_offline(self) -> ToolRouter<TypstlabServer> {
        ToolRouter::new()
            .with_route(ToolRoute::new_dyn(Self::status_attr(), |mut ctx| {
                let server = ctx.service;
                let args_res = Parameters::<StatusArgs>::from_context_part(&mut ctx);
                async move {
                    let Parameters(args) = args_res?;
                    Self::status(server, args).await
                }
                .boxed()
            }))
            .with_route(ToolRoute::new_dyn(
                Self::typst_docs_status_attr(),
                |mut _ctx| {
                    let server = _ctx.service;
                    async move { Self::typst_docs_status(server).await }.boxed()
                },
            ))
    }

    fn cmd_generate_attr() -> Tool {
        Tool::new(
            Cow::Borrowed("cmd_generate"),
            "Generate paper artifacts",
            rmcp::handler::server::common::schema_for_type::<CmdGenerateArgs>(),
        )
        .with_safety(Safety {
            network: true,
            reads: true,
            writes: true,
            writes_sot: false,
        })
    }

    fn status_attr() -> Tool {
        Tool::new(
            Cow::Borrowed("cmd_status"),
            "Get project status report",
            rmcp::handler::server::common::schema_for_type::<StatusArgs>(),
        )
        .with_safety(Safety {
            network: false,
            reads: true,
            writes: false,
            writes_sot: false,
        })
    }

    fn build_attr() -> Tool {
        Tool::new(
            Cow::Borrowed("cmd_build"),
            "Build paper using Typst",
            rmcp::handler::server::common::schema_for_type::<BuildArgs>(),
        )
        .with_safety(Safety {
            network: true,
            reads: true,
            writes: true,
            writes_sot: false,
        })
    }

    fn typst_docs_status_attr() -> Tool {
        Tool::new(
            Cow::Borrowed("cmd_typst_docs_status"),
            "Get Typst docs sync status",
            rmcp::handler::server::common::schema_for_type::<TypstDocsStatusArgs>(),
        )
        .with_safety(Safety {
            network: false,
            reads: true,
            writes: false,
            writes_sot: false,
        })
    }

    pub async fn cmd_generate(
        server: &TypstlabServer,
        args: CmdGenerateArgs,
    ) -> Result<CallToolResult, McpError> {
        let project_root = server.context.project_root.clone();
        validate_paper_id(&args.paper_id)?;
        let paper_id = args.paper_id.clone();
        run_blocking(move || {
            let project = Project::load(project_root).map_err(errors::from_core_error)?;
            generate_paper(&project, &paper_id).map_err(errors::from_core_error)?;
            Ok(())
        })
        .await?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Generated paper: {}",
            args.paper_id
        ))]))
    }

    pub async fn status(
        server: &TypstlabServer,
        args: StatusArgs,
    ) -> Result<CallToolResult, McpError> {
        let project_root = server.context.project_root.clone();
        if let Some(id) = &args.paper_id {
            validate_paper_id(id)?;
        }
        let paper_id = args.paper_id.clone();
        let report = run_blocking(move || {
            let project = Project::load(project_root).map_err(errors::from_core_error)?;
            let engine = StatusEngine::new();
            Ok(engine.run(&project, paper_id.as_deref()))
        })
        .await?;
        let json = serde_json::to_string_pretty(&report).map_err(errors::from_display)?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    pub async fn build(
        server: &TypstlabServer,
        args: BuildArgs,
    ) -> Result<CallToolResult, McpError> {
        let project_root = server.context.project_root.clone();
        validate_paper_id(&args.paper_id)?;
        let paper_id = args.paper_id.clone();
        let full = args.full;
        let outcome = run_blocking(move || build_blocking(project_root, paper_id, full)).await?;

        if !outcome.success {
            return Err(build_failed_error(outcome.exit_code, &outcome.stderr));
        }

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Built '{}' to {}",
            args.paper_id,
            outcome.output_path.display()
        ))]))
    }

    pub async fn typst_docs_status(server: &TypstlabServer) -> Result<CallToolResult, McpError> {
        let project_root = server.context.project_root.clone();
        let res = run_blocking(move || docs_status_blocking(project_root)).await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&res).map_err(errors::from_display)?,
        )]))
    }
}

fn build_failed_error(exit_code: i32, stderr: &str) -> McpError {
    errors::build_failed(format!(
        "Typst compilation failed with exit code {}.\n\nError output:\n{}",
        exit_code, stderr
    ))
}

struct BuildOutcome {
    output_path: std::path::PathBuf,
    success: bool,
    stderr: String,
    exit_code: i32,
}

async fn run_blocking<T, F>(f: F) -> Result<T, McpError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, McpError> + Send + 'static,
{
    let permit = blocking_semaphore()
        .acquire_owned()
        .await
        .map_err(errors::from_display)?;

    spawn_blocking(move || {
        let _permit = permit;
        f()
    })
    .await
    .map_err(errors::from_display)?
}

fn blocking_semaphore() -> Arc<Semaphore> {
    static SEM: OnceLock<Arc<Semaphore>> = OnceLock::new();
    SEM.get_or_init(|| Arc::new(Semaphore::new(4))).clone()
}

fn build_blocking(
    project_root: std::path::PathBuf,
    paper_id: String,
    full: bool,
) -> Result<BuildOutcome, McpError> {
    let project = Project::load(project_root).map_err(errors::from_core_error)?;
    let paper = find_paper(&project, &paper_id)?;
    let output_name = paper.config().output.name.as_str();

    validate_output_name(output_name)?;
    ensure_generated(&project, paper, &paper_id, full)?;

    let output_path = build_output_path(&project.root, &paper_id, output_name)?;
    let main_file_path = paper.absolute_main_file_path();
    let typst_args = build_typst_args(paper, &main_file_path, &output_path);

    let start_time = Utc::now();
    let exec_result = exec_typst(ExecOptions {
        project_root: project.root.clone(),
        args: typst_args,
        required_version: project.config().typst.version.clone(),
    })
    .map_err(errors::from_core_error)?;
    let finish_time = Utc::now();

    record_build_state(
        &project.root,
        &paper_id,
        &output_path,
        &exec_result,
        start_time,
        finish_time,
    )?;

    Ok(BuildOutcome {
        output_path,
        success: exec_result.exit_code == 0,
        stderr: exec_result.stderr,
        exit_code: exec_result.exit_code,
    })
}

fn validate_output_name(output_name: &str) -> Result<(), McpError> {
    is_safe_single_component(Path::new(output_name)).map_err(|err| {
        errors::invalid_params(format!("Invalid output name '{}': {err}", output_name))
    })
}

fn validate_paper_id(paper_id: &str) -> Result<(), McpError> {
    typstlab_core::path::validate_paper_id(paper_id).map_err(errors::from_core_error)
}

fn find_paper<'a>(project: &'a Project, paper_id: &str) -> Result<&'a Paper, McpError> {
    project
        .find_paper(paper_id)
        .ok_or_else(|| errors::invalid_params(format!("Paper '{}' not found", paper_id)))
}

fn ensure_generated(
    project: &Project,
    paper: &Paper,
    paper_id: &str,
    full: bool,
) -> Result<(), McpError> {
    if full || !paper.generated_dir().exists() {
        generate_paper(project, paper_id).map_err(errors::from_core_error)?;
    }
    Ok(())
}

fn build_output_path(
    project_root: &Path,
    paper_id: &str,
    output_name: &str,
) -> Result<std::path::PathBuf, McpError> {
    let dist_dir = project_root.join("dist").join(paper_id);
    std::fs::create_dir_all(&dist_dir).map_err(errors::from_display)?;
    Ok(dist_dir.join(format!("{}.pdf", output_name)))
}

fn build_typst_args(paper: &Paper, main_file: &Path, output_path: &Path) -> Vec<String> {
    let mut typst_args = vec!["compile".to_string()];
    if let Some(root_dir) = paper.typst_root_dir() {
        typst_args.push("--root".to_string());
        typst_args.push(root_dir.display().to_string());
    }
    typst_args.push(main_file.display().to_string());
    typst_args.push(output_path.display().to_string());
    typst_args
}

fn record_build_state(
    project_root: &Path,
    paper_id: &str,
    output_path: &Path,
    exec_result: &ExecResult,
    started_at: chrono::DateTime<Utc>,
    finished_at: chrono::DateTime<Utc>,
) -> Result<(), McpError> {
    let duration_ms = (finished_at - started_at).num_milliseconds().max(0) as u64;
    let build_state = BuildState {
        last: Some(LastBuild {
            paper: paper_id.to_string(),
            success: exec_result.exit_code == 0,
            started_at,
            finished_at,
            duration_ms,
            output: output_path.to_path_buf(),
            error: if exec_result.exit_code == 0 {
                None
            } else {
                Some(exec_result.stderr.clone())
            },
        }),
    };

    let state_path = project_root.join(".typstlab/state.json");
    let mut state = State::load_or_empty(state_path.clone());
    state.build = Some(build_state);
    state.save(state_path).map_err(errors::from_core_error)?;
    Ok(())
}

fn docs_status_blocking(project_root: std::path::PathBuf) -> Result<serde_json::Value, McpError> {
    let docs_dir = project_root.join(".typstlab/kb/typst/docs");
    let docs_present = docs_dir.exists();
    let state = State::load_or_empty(project_root.join(".typstlab/state.json"));
    let info = state.docs.and_then(|docs| docs.typst);
    Ok(serde_json::json!({
        "present": docs_present,
        "version": info.as_ref().map(|item| item.version.clone()),
        "synced_at": info.as_ref().map(|item| item.synced_at.to_rfc3339()),
        "source": info.as_ref().map(|item| item.source.clone()),
        "path": docs_present.then(|| docs_dir.display().to_string()),
    }))
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct CmdGenerateArgs {
    pub paper_id: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct StatusArgs {
    pub paper_id: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct BuildArgs {
    pub paper_id: String,
    #[serde(default)]
    pub full: bool,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct TypstDocsStatusArgs {}

#[cfg(test)]
mod tests {
    use super::{CmdTool, build_failed_error, validate_output_name};
    use rmcp::model::Tool;

    #[test]
    fn test_validate_output_name_rejects_multiple_components() {
        let result = validate_output_name("foo/bar");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_output_name_rejects_rooted_path() {
        let result = validate_output_name("/tmp");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_output_name_accepts_simple_name() {
        let result = validate_output_name("paper");
        assert!(result.is_ok());
    }

    fn is_network_enabled(tool: Tool) -> bool {
        tool.annotations
            .and_then(|a| a.open_world_hint)
            .unwrap_or(false)
    }

    #[test]
    fn test_build_and_generate_tools_mark_network_usage() {
        assert!(is_network_enabled(CmdTool::build_attr()));
        assert!(is_network_enabled(CmdTool::cmd_generate_attr()));
    }

    #[test]
    fn test_build_output_path_creates_dist_dir() {
        let temp = typstlab_testkit::temp_dir_in_workspace();
        let dist = temp.path().join("dist");
        assert!(!dist.exists());

        let path = super::build_output_path(temp.path(), "paper", "out").unwrap();
        assert!(path.parent().unwrap().exists());
        assert!(path.parent().unwrap().ends_with("dist/paper"));
    }

    #[test]
    fn test_docs_status_blocking_reports_presence() {
        let temp = typstlab_testkit::temp_dir_in_workspace();
        let empty = super::docs_status_blocking(temp.path().to_path_buf()).unwrap();
        assert!(!empty["present"].as_bool().unwrap());
        assert!(empty["path"].is_null());

        let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
        std::fs::create_dir_all(&docs_dir).unwrap();
        let populated = super::docs_status_blocking(temp.path().to_path_buf()).unwrap();
        assert!(populated["present"].as_bool().unwrap());
        assert_eq!(
            populated["path"].as_str().unwrap(),
            docs_dir.display().to_string()
        );
    }

    #[test]
    fn test_build_failed_error_sets_standard_code() {
        let err = build_failed_error(42, "boom");
        assert_eq!(err.code, rmcp::model::ErrorCode(-32004)); // BUILD_FAILED
        let data = err.data.unwrap();
        assert_eq!(data["code"], crate::errors::BUILD_FAILED);
        assert!(err.message.contains("42"));
        assert!(err.message.contains("boom"));
    }
}
