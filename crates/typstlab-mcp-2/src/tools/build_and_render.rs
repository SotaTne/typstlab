use rmcp::model::CallToolResult;
use typstlab_app::{AppContext, BuildAction};
use typstlab_proto::Entity;

pub fn execute(ctx: AppContext, paper_id: String) -> Result<CallToolResult, String> {
    use typstlab_base::driver::TypstDriver;
    let driver = TypstDriver::new(ctx.typst.path());
    let _action = BuildAction::new(ctx.loaded_project, driver, Some(vec![paper_id]));

    todo!()
}
