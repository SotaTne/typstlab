/// アクション（動き）のプロトコル
pub trait Action<Output, Event, Warning, Error>
where
    Error: std::error::Error,
{
    fn run(
        self,
        monitor: &mut dyn FnMut(Event),
        warning: &mut dyn FnMut(Warning),
    ) -> Result<Output, Vec<Error>>;
}
