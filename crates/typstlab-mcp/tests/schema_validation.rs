use typstlab_mcp::context::McpContext;
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;

#[tokio::test]
async fn test_all_tools_have_object_input_schema() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let tools = server.tool_router.list_all();
    let mut failures = Vec::new();

    for (i, tool) in tools.iter().enumerate() {
        let schema_json = serde_json::to_value(&tool.input_schema).unwrap();
        let schema_type = schema_json.get("type").and_then(|v| v.as_str());

        if schema_type != Some("object") {
            failures.push(format!(
                "Tool at index {} ({}) has invalid inputSchema type: {:?}. Schema: {}",
                i,
                tool.name,
                schema_type,
                serde_json::to_string(&tool.input_schema).unwrap()
            ));
        }
    }

    if !failures.is_empty() {
        panic!("Schema validation failures:\n{}", failures.join("\n"));
    }
}
