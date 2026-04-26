pub trait CliSpeaker<Event, Warning, Error, Output> {
    fn render_event(&self, event: Event);
    fn render_warning(&self, warning: Warning);
    fn render_error(&self, error: &Error);
    fn render_result(&self, output: &Output);
}

/// AIエージェント向けの Speaker
pub trait McpSpeaker<Event, Warning, Error, Output> {
    fn render_event(&self, event: Event) -> String;
    fn render_warning(&self, warning: Warning) -> String;
    fn render_error(&self, error: &Error) -> String;
    fn render_result(&self, output: &Output) -> String;
}
