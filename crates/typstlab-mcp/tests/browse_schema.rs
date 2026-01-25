//! browse系スキーマ統一テスト（DESIGN.md 5.10.5）

use serde_json::Value;
use typstlab_mcp::context::McpContext;
use typstlab_mcp::handlers::docs::DocsTool;
use typstlab_mcp::handlers::rules::RulesTool;
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;

#[tokio::test]
async fn test_docs_browse_missing_returns_schema() {
    // 存在しないパスでmissing=trueを返すこと（エラーではない）
    // DESIGN.md 5.10.5: browse系は {items: [], missing: bool, truncated?: bool}
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::test_docs_browse(
        &server,
        typstlab_mcp::handlers::docs::DocsBrowseArgs {
            path: Some("nonexistent/path".to_string()),
        },
    )
    .await
    .expect("Should return OK, not error");

    let text = res.content[0].as_text().unwrap();
    let json: Value = serde_json::from_str(&text.text).unwrap();

    assert_eq!(json["missing"].as_bool().unwrap(), true);
    assert_eq!(json["items"].as_array().unwrap().len(), 0);
    assert_eq!(json["truncated"].as_bool().unwrap(), false);
}

#[tokio::test]
async fn test_docs_browse_root_missing() {
    // docsディレクトリ自体が存在しない場合もmissing=trueを返す
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::test_docs_browse(
        &server,
        typstlab_mcp::handlers::docs::DocsBrowseArgs {
            path: Some("subdir".to_string()),
        },
    )
    .await
    .expect("Should return OK, not error");

    let text = res.content[0].as_text().unwrap();
    let json: Value = serde_json::from_str(&text.text).unwrap();

    assert_eq!(json["missing"].as_bool().unwrap(), true);
}

#[tokio::test]
async fn test_rules_browse_missing_returns_schema() {
    // rules_browseも同様にmissing=trueを返す
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_browse(
        &server,
        typstlab_mcp::handlers::rules::RulesBrowseArgs {
            path: "rules/nonexistent".to_string(),
        },
    )
    .await
    .expect("Should return OK, not error");

    let text = res.content[0].as_text().unwrap();
    let json: Value = serde_json::from_str(&text.text).unwrap();

    assert_eq!(json["missing"].as_bool().unwrap(), true);
    assert_eq!(json["items"].as_array().unwrap().len(), 0);
    // browse系はtruncatedはオプション（書いても書かなくてもOK）
}

#[tokio::test]
async fn test_browse_existing_no_missing_flag() {
    // 存在するパスではmissingフラグが無い（またはfalse）
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::test_docs_browse(
        &server,
        typstlab_mcp::handlers::docs::DocsBrowseArgs {
            path: None, // root
        },
    )
    .await
    .expect("Should return OK");

    let text = res.content[0].as_text().unwrap();
    let json: Value = serde_json::from_str(&text.text).unwrap();

    // missingフラグは無いか、falseであること
    assert!(json.get("missing").is_none() || json["missing"].as_bool().unwrap() == false);
    assert!(json["items"].is_array());
}
