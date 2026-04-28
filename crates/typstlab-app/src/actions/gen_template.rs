use crate::actions::create::{CreateAction, CreateEvent};
use crate::models::{CollectionError, Project, ProjectConfig, ProjectHandle, Template};
use std::path::PathBuf;
use thiserror::Error;
use typstlab_proto::{Action, AppEvent, Entity, EventScope, Loaded};

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

impl Action for GenTemplateAction {
    type Output = ();
    type Event = GenTemplateEvent;
    type Warning = ();
    type Error = GenTemplateError;

    fn run(
        self,
        monitor: &mut dyn FnMut(AppEvent<GenTemplateEvent>),
        _warning: &mut dyn FnMut(Self::Warning),
    ) -> Result<Self::Output, Vec<Self::Error>> {
        let scope = EventScope::labeled("gen_template", self.template_id.clone());
        let templates_dir = self.project.templates_scope().path();
        let target = Template::new(self.template_id.clone(), templates_dir);

        let create_action = CreateAction { target, args: () };

        let loaded = create_action
            .run(
                &mut |e| monitor(e.map_payload(GenTemplateEvent::CreatingTemplate)),
                &mut |_| {},
            )
            .map_err(|errors| vec![GenTemplateError::CreateFailed(errors)])?;

        monitor(AppEvent::line(
            scope,
            GenTemplateEvent::TemplateReady {
                path: loaded.actual.path(),
            },
        ));

        Ok(())
    }
}
