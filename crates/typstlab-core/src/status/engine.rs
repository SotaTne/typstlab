use crate::{error::TypstlabError, project::Project, status::schema::StatusReport};

/// Statusを組み立てる唯一の入口（checksの依存地獄を防ぐための集約点）
pub struct StatusEngine;

impl StatusEngine {
    pub fn run(_project: &Project, _paper_id: Option<&str>) -> Result<StatusReport, TypstlabError> {
        // TODO: checks を呼んで StatusReport を組み立てる
        Ok(StatusReport::empty())
    }
}
