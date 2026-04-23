use crate::actions::create::{CreateAction, CreateEvent};
use crate::models::{CollectionError, Project, ProjectConfig, ProjectHandle, Template};
use std::path::PathBuf;
use thiserror::Error;
use typstlab_proto::{Action, Entity, Loaded};

#[derive(Error, Debug)]
pub enum GenTemplateError {
    #[error("Template resolution failed: {0}")]
    ResolveFailed(#[from] CollectionError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Creation process failed: {0:?}")]
    CreateFailed(Vec<crate::actions::create::CreateError>),
}

#[derive(Debug, Clone)]
pub enum GenTemplateEvent {
    CreatingTemplate(CreateEvent),
    TemplateReady { path: PathBuf },
}

pub struct GenTemplateAction {
    pub project: Loaded<Project, ProjectConfig>,
    pub template_id: String,
}

impl Action<(), GenTemplateEvent, (), GenTemplateError> for GenTemplateAction {
    fn run(
        self,
        monitor: &mut dyn FnMut(GenTemplateEvent),
        _warning: &mut dyn FnMut(()),
    ) -> Result<(), Vec<GenTemplateError>> {
        let templates_dir = self.project.templates_scope().path();
        let target = Template::new(self.template_id.clone(), templates_dir);

        let create_action = CreateAction { target, args: () };

        let loaded = create_action
            .run(
                &mut |e| monitor(GenTemplateEvent::CreatingTemplate(e)),
                &mut |_| {},
            )
            .map_err(|errors| vec![GenTemplateError::CreateFailed(errors)])?;

        monitor(GenTemplateEvent::TemplateReady {
            path: loaded.actual.path(),
        });

        Ok(())
    }
}
