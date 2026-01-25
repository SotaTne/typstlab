use typstlab_mcp::context::McpContext;
use typstlab_mcp::handlers::cmd::{BuildArgs, CmdGenerateArgs, CmdTool, StatusArgs};
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;

#[tokio::test]
async fn test_cmd_build_rejects_path_traversal() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = CmdTool::test_cmd_build(
        &server,
        BuildArgs {
            paper_id: "../etc".to_string(), // Traversal
            full: false,
        },
    )
    .await;

    assert!(res.is_err(), "Should reject traversal paper_id");
    let err = res.unwrap_err();
    assert!(
        err.message.contains("PAPER_INVALID_ID") || err.message.contains("Invalid paper ID"),
        "Error should indicate invalid ID: {:?}",
        err
    );
}

#[tokio::test]
async fn test_cmd_generate_rejects_absolute_path() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = CmdTool::test_cmd_generate(
        &server,
        CmdGenerateArgs {
            paper_id: "/tmp/foo".to_string(), // Absolute
        },
    )
    .await;

    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.message.contains("PAPER_INVALID_ID") || err.message.contains("Invalid paper ID"));
}

#[tokio::test]
async fn test_status_rejects_invalid_paper_id() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = CmdTool::test_status(
        &server,
        StatusArgs {
            paper_id: Some("foo/bar".to_string()), // Multiple components
        },
    )
    .await;

    assert!(res.is_err());
}
