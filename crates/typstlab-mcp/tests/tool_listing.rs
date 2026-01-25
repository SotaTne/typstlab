//! 公開ツールセットの検証テスト
//!
//! DESIGN.md 5.10.1および5.10.4の仕様に基づき、
//! online/offlineモードで公開されるツールが正しいことを検証する。

use typstlab_mcp::context::McpContext;
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;

#[tokio::test]
async fn test_rules_get_in_list_all() {
    // ユーザーの最新の要求により rules_get は公開ツールとなった
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let tools = server.tool_router.list_all();
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    assert!(
        tool_names.contains(&"rules_get"),
        "rules_get should be in list_all"
    );
}

#[tokio::test]
async fn test_docs_get_in_list_all() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let tools = server.tool_router.list_all();
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    assert!(
        tool_names.contains(&"docs_get"),
        "docs_get should be in list_all"
    );
}

#[tokio::test]
async fn test_public_tools_match_design_online() {
    // onlineモードの公開ツールがDESIGN.md仕様と一致すること
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let tools = server.tool_router.list_all();
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    // DESIGN.md 5.10.1で定義されたonlineモードのツール
    let expected_tools = vec![
        "rules_browse",
        "rules_search",
        "rules_page",
        "rules_get",
        "docs_browse",
        "docs_search",
        "docs_get",
        "cmd_generate",
        "cmd_build",
        "cmd_status",
        "cmd_typst_docs_status",
    ];

    for expected in &expected_tools {
        assert!(
            tool_names.contains(expected),
            "Missing tool in online mode: {}",
            expected
        );
    }
}

#[tokio::test]
async fn test_public_tools_match_design_offline() {
    // offlineモードでcmd_generate/cmd_buildが除外されること
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    // offlineモードで初期化
    let server = TypstlabServer::new(ctx, true);

    let tools = server.tool_router.list_all();
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    // offlineで使えるツール
    let expected_tools = vec![
        "rules_browse",
        "rules_search",
        "rules_page",
        "rules_get",
        "docs_browse",
        "docs_search",
        "docs_get",
        "cmd_status",
        "cmd_typst_docs_status",
    ];

    for expected in &expected_tools {
        assert!(
            tool_names.contains(expected),
            "Missing tool in offline mode: {}",
            expected
        );
    }
}
