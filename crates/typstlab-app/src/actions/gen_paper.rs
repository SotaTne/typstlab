use crate::actions::create::{CreateAction, CreateEvent};
use crate::actions::discovery::{DiscoveryAction, DiscoveryError};
use crate::models::{
    CollectionError, Paper, PaperCreationArgs, PaperError, Project, ProjectConfig, ProjectHandle,
};
use std::path::PathBuf;
use thiserror::Error;
use typstlab_base::driver::{TypstCommand, TypstDriver};
use typstlab_proto::{Action, AppEvent, Entity, EventScope, Loaded};

#[derive(Error, Debug)]
pub enum GenPaperError {
    #[error("Discovery failure: {0:?}")]
    Discovery(Vec<DiscoveryError>),
    #[error("Template resolution failed: {0}")]
    ResolveFailed(#[from] CollectionError),
    #[error("Paper creation failed: {source}")]
    PaperError {
        #[from]
        source: PaperError,
    },
    #[error("Creation process failed: {0:?}")]
    CreateFailed(Vec<crate::actions::create::CreateError>),
    #[error("Template not found locally and Typst Init failed: {0}")]
    TemplateOrInitFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub enum GenPaperEvent {
    ResolvingTemplate { id: String },
    ResolvedLocal { path: PathBuf },
    FallbackToInit { template_input: String },
    CreatingPaper(CreateEvent),
    PaperReady { path: PathBuf },
}

pub struct GenPaperAction {
    pub project: Loaded<Project, ProjectConfig>,
    pub paper_id: String,
    pub template_input: Option<String>,
    pub typst_driver: TypstDriver,
}

impl Action for GenPaperAction {
    type Output = ();
    type Event = GenPaperEvent;
    type Warning = ();
    type Error = GenPaperError;

    fn run(
        self,
        monitor: &mut dyn FnMut(AppEvent<GenPaperEvent>),
        _warning: &mut dyn FnMut(Self::Warning),
    ) -> Result<Self::Output, Vec<Self::Error>> {
        let scope = EventScope::labeled("gen_paper", self.paper_id.clone());
        // 1. テンプレートの特定 (ローカルのみ)
        let local_template = if let Some(t_input) = &self.template_input {
            monitor(AppEvent::line(
                scope.clone(),
                GenPaperEvent::ResolvingTemplate {
                    id: t_input.clone(),
                },
            ));

            let discovery =
                DiscoveryAction::new(self.project.templates_scope(), vec![t_input.clone()]);

            match discovery.run(&mut |_| {}, &mut |_| {}) {
                Ok(templates) => {
                    let t = templates.into_iter().next();
                    if let Some(ref local) = t {
                        monitor(AppEvent::line(
                            scope.clone(),
                            GenPaperEvent::ResolvedLocal { path: local.path() },
                        ));
                    } else {
                        monitor(AppEvent::line(
                            scope.clone(),
                            GenPaperEvent::FallbackToInit {
                                template_input: t_input.clone(),
                            },
                        ));
                    }
                    t
                }
                Err(errs) => {
                    // NotFound 以外の本物のエラー（I/Oエラー等）が含まれている場合はフォールバックせずに失敗させる
                    let has_real_error = errs
                        .iter()
                        .any(|e| matches!(e, DiscoveryError::ResolveFailed { .. }));
                    if has_real_error {
                        return Err(vec![GenPaperError::Discovery(errs)]);
                    }

                    monitor(AppEvent::line(
                        scope.clone(),
                        GenPaperEvent::FallbackToInit {
                            template_input: t_input.clone(),
                        },
                    ));
                    None
                }
            }
        } else {
            None
        };

        // Paper のパスを事前に決定
        let paper = Paper::new(self.paper_id.clone(), self.project.papers_scope().path());
        let dest_path = paper.path();

        // 2. テンプレートの適用 (または init) - 先に実行することでディレクトリ空制約を回避し、テンプレート展開結果を優先する
        if let Some(t_input) = &self.template_input {
            if let Some(local) = local_template {
                // ローカルコピー
                copy_dir_recursive(&local.path(), &dest_path)
                    .map_err(|e| vec![GenPaperError::Io(e)])?;
            } else {
                // typst init 実行
                let command = TypstCommand::Init {
                    template: t_input.clone(),
                    output: Some(dest_path.clone()),
                };
                let result = self
                    .typst_driver
                    .execute(command)
                    .map_err(|e| vec![GenPaperError::TemplateOrInitFailed(e.to_string())])?;
                if result.exit_code != 0 {
                    return Err(vec![GenPaperError::TemplateOrInitFailed(result.stderr)]);
                }
            }
        }

        // 3. Paper の物理生成 (CreateAction を利用) - 後から実行することで、paper.toml 等を追記し、既存の main.typ と競合させない
        let create_action = CreateAction {
            target: paper,
            args: PaperCreationArgs {
                title: self.paper_id.clone(),
            },
        };

        let loaded_paper = create_action
            .run(
                &mut |e| monitor(e.map_payload(GenPaperEvent::CreatingPaper)),
                &mut |_| {},
            )
            .map_err(|errors| vec![GenPaperError::CreateFailed(errors)])?;

        monitor(AppEvent::line(
            scope,
            GenPaperEvent::PaperReady {
                path: loaded_paper.actual.path(),
            },
        ));

        Ok(())
    }
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}
