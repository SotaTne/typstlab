use rmcp::model::Tool;
use serde_json::json;

pub mod cmd;
pub mod common;
pub mod docs;
pub mod rules;

#[derive(Debug, Clone, Copy, Default)]
pub struct Safety {
    pub network: bool,
    pub reads: bool,
    pub writes: bool,
    pub writes_sot: bool,
}

pub trait ToolExt {
    fn with_safety(self, safety: Safety) -> Tool;
}

impl ToolExt for Tool {
    fn with_safety(mut self, safety: Safety) -> Tool {
        // Map to RMCP annotations
        let mut annotations = self.annotations.unwrap_or_default();
        annotations.read_only_hint = Some(!safety.writes);
        annotations.open_world_hint = Some(safety.network);
        self.annotations = Some(annotations);

        // Add custom safety meta
        let mut meta = self.meta.unwrap_or_default();
        meta.insert(
            "safety".to_string(),
            json!({
                "network": safety.network,
                "reads": safety.reads,
                "writes": safety.writes,
                "writes_sot": safety.writes_sot,
            }),
        );
        self.meta = Some(meta);
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize)]
pub struct LineRange {
    pub start: usize,
    pub end: usize, // end is exclusive
}
