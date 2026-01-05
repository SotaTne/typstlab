
impl StatusReport {
    pub fn empty() -> Self {
        Self {
            schema_version: "1.0".to_string(),
            project: None,
            time: None,
            summary: None,
            checks: vec![],
            actions: vec![],
        }
    }
}
