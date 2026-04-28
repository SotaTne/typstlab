use rmcp::ErrorData as McpError;
use std::path::PathBuf;
use typstlab_app::AppContext;
use typstlab_app::actions::bootstrap::BootstrapAction;
use typstlab_proto::Action;

pub fn bootstrap_context(project_root: PathBuf) -> Result<AppContext, String> {
    let cache_root = dirs::cache_dir()
        .ok_or_else(|| "could not find cache directory".to_string())?
        .join("typstlab");

    BootstrapAction {
        project_root,
        cache_root,
    }
    .run(&mut |_| {}, &mut |_| {})
    .map_err(format_bootstrap_errors)
}

pub fn internal_error(message: impl Into<String>) -> McpError {
    McpError::internal_error(message.into(), None)
}

fn format_bootstrap_errors(errors: Vec<typstlab_app::BootstrapError>) -> String {
    errors
        .into_iter()
        .map(|error| error.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}
