use crate::AppEvent;
use std::fmt::Debug;

pub trait CliSpeaker {
    type Event: Clone + Debug + 'static;
    type Warning;
    type Error;
    type Output;

    fn render_event(&self, event: AppEvent<Self::Event>);
    fn render_warning(&self, warning: Self::Warning);
    fn render_error(&self, error: &Self::Error);
    fn render_result(&self, output: &Self::Output);
}

/// AIエージェント向けの Speaker
pub trait McpSpeaker {
    type Event: Clone + Debug + 'static;
    type Warning;
    type Error;
    type Output;

    fn render_event(&self, event: AppEvent<Self::Event>) -> String;
    fn render_warning(&self, warning: Self::Warning) -> String;
    fn render_error(&self, error: &Self::Error) -> String;
    fn render_result(&self, output: &Self::Output) -> String;
}
