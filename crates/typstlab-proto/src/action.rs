use crate::AppEvent;
use std::fmt::Debug;

/// アクション（動き）のプロトコル
pub trait Action {
    type Output;
    type Event: Clone + Debug + 'static;
    type Warning;
    type Error: std::error::Error;

    fn run(
        self,
        monitor: &mut dyn FnMut(AppEvent<Self::Event>),
        warning: &mut dyn FnMut(Self::Warning),
    ) -> Result<Self::Output, Vec<Self::Error>>;
}
