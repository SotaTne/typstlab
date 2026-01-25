//! 公開ツールセットの検証テスト
//!
//! DESIGN.md 5.10.1および5.10.4の仕様に基づき、
//! online/offlineモードで公開されるツールが正しいことを検証する。

use typstlab_mcp::context::McpContext;
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;

#[tokio::test]
async fn test_rules_get_not_in_list_all() {
    // rules_getはDESIGN.md仕様に含まれないため、list_allに出現してはならない
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let tools = server.tool_router.list_all();
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    assert!(
        !tool_names.contains(&"rules_get"),
        "rules_get should not be in list_all (not in DESIGN.md v0.1 spec)"
    );
}

#[tokio::test]
async fn test_rules_get_call_fails() {
    // rules_getを呼び出そうとすると「ツール不明」エラーになるべき
    // エラーメッセージの内容は問わず、エラーが返ることを確認
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // MCPプロトコルでcall_toolを試行
    // ツールが存在しない場合、RMCPがエラーを返す
    // 実装依存なのでエラーの詳細は問わない

    let tools = server.tool_router.list_all();
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    // リストに無いことを再確認
    assert!(!tool_names.contains(&"rules_get"));

    // TODO: call_toolの実際の呼び出しテストはMCPプロトコルレベルで実施
    // ここではlist_allに無いことを確認するに留める
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
        "rules_list",
        "docs_browse",
        "docs_search",
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

    // rules_getは含まれない
    assert!(!tool_names.contains(&"rules_get"));
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
        "rules_list",
        "docs_browse",
        "docs_search",
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

    // cmd_generate/cmd_buildは含まれない
    assert!(
        !tool_names.contains(&"cmd_generate"),
        "cmd_generate should not be available offline"
    );
    assert!(
        !tool_names.contains(&"cmd_build"),
        "cmd_build should not be available offline"
    );

    // rules_getも含まれない
    assert!(!tool_names.contains(&"rules_get"));
}
