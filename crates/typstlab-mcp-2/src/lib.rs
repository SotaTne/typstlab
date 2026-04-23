use typstlab_app::actions::build::{BuildEvent, BuildError};
use typstlab_proto::McpSpeaker;

pub struct McpBuildPresenter;

impl McpSpeaker<BuildEvent, BuildError, ()> for McpBuildPresenter {
    fn render_event(&self, event: BuildEvent) -> String {
        match event {
            BuildEvent::ProjectLoaded { name } => {
                format!("Project loaded: {}", name)
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
            BuildEvent::Finished { paper_id, output_path } => {
                format!("SUCCESS: Build for '{}' completed. Output: {}", paper_id, output_path.display())
            }
        }
    }

    fn render_error(&self, error: &BuildError) -> String {
        match error {
            BuildError::PaperBuildError { paper_id, error } => {
                format!("ERROR in paper '{}':\n{}", paper_id, error)
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
