use typstlab_app::actions::build::{BuildEvent, BuildError};
use typstlab_proto::{McpSpeaker, Artifact};

pub struct McpBuildPresenter;

impl McpSpeaker<BuildEvent, BuildError, ()> for McpBuildPresenter {
    fn render_event(&self, event: BuildEvent) -> String {
        match event {
            BuildEvent::ProjectLoaded { name } => {
                format!("Project loaded: {}", name)
            }
            BuildEvent::DiscoveryStarted { inputs } => {
                format!("Identifying targets for inputs: {:?}", inputs)
            }
            BuildEvent::ResolvingTypst { version } => {
                format!("Checking Typst version: {}", version)
            }
            BuildEvent::DiscoveredTargets { count } => {
                format!("Discovered {} papers to build.", count)
            }
            BuildEvent::Starting { paper_id } => {
                format!("Starting build for paper: {}", paper_id)
            }
            BuildEvent::Finished { artifact, duration_ms } => {
                format!("SUCCESS: Artifact created at '{}' in {}ms.", artifact.root().display(), duration_ms)
            }
        }
    }

    fn render_error(&self, error: &BuildError) -> String {
        match error {
            BuildError::PaperBuildError(artifact) => {
                let artifact_error = artifact.error();
                let error_message = artifact_error
                    .as_deref()
                    .unwrap_or("artifact reported no error message");
                format!(
                    "ERROR in artifact '{}':\n{}", 
                    artifact.root().display(), 
                    error_message
                )
            }
            _ => {
                format!("SYSTEM ERROR: {}", error)
            }
        }
    }

    fn render_result(&self, _output: &()) -> String {
        "All build tasks finished successfully.".to_string()
    }
}
