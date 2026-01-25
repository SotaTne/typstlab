//! DESIGN.md 5.10.5 の仕様準拠テスト
//!
//! 仕様: スキーマ統一ポリシー（search系/browse系の一貫性、missing時の構造維持）
//! 根拠: DESIGN.md 5.10.5 "スキーマ統一ポリシー"

use serde_json::Value;
use typstlab_mcp::context::McpContext;
use typstlab_mcp::handlers::docs::{DocsSearchArgs, DocsTool};
use typstlab_mcp::handlers::rules::{RulesBrowseArgs, RulesSearchArgs, RulesTool};
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;

fn parse_result(result: &rmcp::model::CallToolResult) -> Value {
    let text = result.content[0].as_text().unwrap();
    serde_json::from_str(&text.text).unwrap()
}

#[tokio::test]
async fn test_search_schema_structure_fixed() {
    // rules_searchがmissing時も固定構造を返すか
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_search(
        &server,
        RulesSearchArgs {
            query: "test".to_string(),
            paper_id: None,
            include_root: true,
        },
    )
    .await
    .unwrap();

    let json = parse_result(&res);

    // 必須フィールドの確認
    assert!(json.get("matches").is_some());
    assert!(json.get("truncated").is_some());
    assert!(json.get("missing").is_some());

    // itemsキー（browse用）が無いこと
    assert!(json.get("items").is_none());
}

#[tokio::test]
async fn test_browse_schema_structure_fixed() {
    // rules_browseがmissing時も固定構造を返すか
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_browse(
        &server,
        RulesBrowseArgs {
            path: "rules".to_string(),
        },
    )
    .await
    .unwrap();

    let json = parse_result(&res);

    // 必須フィールドの確認
    assert!(json.get("items").is_some());
    assert!(json.get("missing").is_some());
    // truncatedはbrowseではオプションだが存在してもよい

    // matchesキー（search用）が無いこと
    assert!(json.get("matches").is_none());
}

#[tokio::test]
async fn test_schema_structure_offline_immutable() {
    // online/offlineで構造が変わらないか
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());

    // Offline mode
    let server_offline = TypstlabServer::new(ctx.clone(), true);
    let res_offline = DocsTool::test_docs_search(
        &server_offline,
        DocsSearchArgs {
            query: "q".to_string(),
        },
    )
    .await
    .unwrap();
    let json_offline = parse_result(&res_offline);

    // Online mode
    let server_online = TypstlabServer::new(ctx, false);
    let res_online = DocsTool::test_docs_search(
        &server_online,
        DocsSearchArgs {
            query: "q".to_string(),
        },
    )
    .await
    .unwrap();
    let json_online = parse_result(&res_online);

    // キー構造が一致することを確認
    assert!(json_offline.is_object());
    assert!(json_online.is_object());

    let keys_offline: Vec<&String> = json_offline.as_object().unwrap().keys().collect();
    let keys_online: Vec<&String> = json_online.as_object().unwrap().keys().collect();

    assert_eq!(keys_offline, keys_online);
}
