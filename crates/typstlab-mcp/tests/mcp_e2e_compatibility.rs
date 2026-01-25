//! DESIGN.md 5.10.10 の仕様準拠テスト
//!
//! 仕様: E2E互換テスト（ツール返却パスのread_resource再入力互換）
//! 根拠: DESIGN.md 5.10.10 "テストマトリクス"

use serde_json::Value;
use typstlab_mcp::context::McpContext;
use typstlab_mcp::handlers::docs::{DocsBrowseArgs, DocsSearchArgs, DocsTool};
use typstlab_mcp::handlers::rules::{RulesBrowseArgs, RulesListArgs, RulesSearchArgs, RulesTool};
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;
// use rmcp::ServerHandler; // Not needed with test_read_resource_by_uri

// Helper to parse content result
fn parse_result(result: &rmcp::model::CallToolResult) -> Value {
    let text = result.content[0].as_text().unwrap();
    serde_json::from_str(&text.text).unwrap()
}

// UTF-8読み取り + 改行コード正規化
fn normalize_content(content: &str) -> String {
    content.replace("\r\n", "\n").replace("\r", "\n")
}

#[tokio::test]
async fn test_e2e_rules_browse_path_to_read_resource_returns_same_content() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    tokio::fs::create_dir_all(&rules_dir).await.unwrap();
    let content_src = "# Guidelines\n\r\nUse UTF-8.";
    tokio::fs::write(rules_dir.join("guide.md"), content_src)
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // 1. rules_browse
    let browse_res = RulesTool::rules_browse(
        &server,
        RulesBrowseArgs {
            path: "rules".to_string(),
        },
    )
    .await
    .unwrap();
    let browse_json = parse_result(&browse_res);
    let path = browse_json["items"][0]["path"].as_str().unwrap();

    let uri = format!("typstlab://rules/{}", path);

    // Use test helper
    let res1 = server
        .test_read_resource_by_uri(&uri)
        .await
        .expect("read_resource failed");

    let content1_utf8 = match &res1.contents[0] {
        rmcp::model::ResourceContents::TextResourceContents { text, .. } => text,
        _ => panic!("Expected text"),
    };

    // 3. read_resource 2回目 (再確認)
    let res2 = server
        .test_read_resource_by_uri(&uri)
        .await
        .expect("read_resource 2 failed");

    let content2_utf8 = match &res2.contents[0] {
        rmcp::model::ResourceContents::TextResourceContents { text, .. } => text,
        _ => panic!("Expected text"),
    };

    // 4. 同一内容確認
    assert_eq!(
        normalize_content(content1_utf8),
        normalize_content(content2_utf8)
    );
    // Also verify it matches source (normalized)
    assert_eq!(
        normalize_content(content1_utf8),
        normalize_content(content_src)
    );
}

#[tokio::test]
async fn test_e2e_rules_search_path_to_read_resource_returns_same_content() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    tokio::fs::create_dir_all(&rules_dir).await.unwrap();
    let content_src = "Search target here";
    tokio::fs::write(rules_dir.join("found.md"), content_src)
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let search_res = RulesTool::rules_search(
        &server,
        RulesSearchArgs {
            query: "target".to_string(),
            paper_id: None,
            include_root: true,
        },
    )
    .await
    .unwrap();
    let search_json = parse_result(&search_res);
    let path = search_json["matches"][0]["path"].as_str().unwrap();

    let uri = format!("typstlab://rules/{}", path);

    let res = server
        .test_read_resource_by_uri(&uri)
        .await
        .expect("read_resource failed");

    let content_utf8 = match &res.contents[0] {
        rmcp::model::ResourceContents::TextResourceContents { text, .. } => text,
        _ => panic!("Expected text"),
    };

    assert_eq!(
        normalize_content(content_utf8),
        normalize_content(content_src)
    );
}

#[tokio::test]
async fn test_e2e_rules_list_path_to_read_resource_returns_same_content() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    tokio::fs::create_dir_all(&rules_dir).await.unwrap();
    let content_src = "List item";
    tokio::fs::write(rules_dir.join("item.md"), content_src)
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let list_res = RulesTool::rules_list(
        &server,
        RulesListArgs {
            paper_id: None,
            include_root: true,
        },
    )
    .await
    .unwrap();
    let list_json = parse_result(&list_res);
    // rules_list returns "files" not "items"
    let path = list_json["files"][0]["path"].as_str().unwrap();

    let uri = format!("typstlab://rules/{}", path);

    let res = server
        .test_read_resource_by_uri(&uri)
        .await
        .expect("read_resource failed");

    let content_utf8 = match &res.contents[0] {
        rmcp::model::ResourceContents::TextResourceContents { text, .. } => text,
        _ => panic!("Expected text"),
    };

    assert_eq!(
        normalize_content(content_utf8),
        normalize_content(content_src)
    );
}

#[tokio::test]
async fn test_e2e_docs_browse_path_to_read_resource_returns_same_content() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();
    let content_src = "Docs content";
    tokio::fs::write(docs_dir.join("intro.md"), content_src)
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let browse_res = DocsTool::test_docs_browse(&server, DocsBrowseArgs { path: None })
        .await
        .unwrap();
    let browse_json = parse_result(&browse_res);

    let returned_path = browse_json["items"][0]["path"].as_str().unwrap();

    let uri = format!("typstlab://docs/{}", returned_path);

    let res = server
        .test_read_resource_by_uri(&uri)
        .await
        .expect("read_resource failed");

    let content_utf8 = match &res.contents[0] {
        rmcp::model::ResourceContents::TextResourceContents { text, .. } => text,
        _ => panic!("Expected text"),
    };

    assert_eq!(
        normalize_content(content_utf8),
        normalize_content(content_src)
    );
}

#[tokio::test]
async fn test_e2e_docs_search_path_to_read_resource_returns_same_content() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();
    let content_src = "Search docs";
    tokio::fs::write(docs_dir.join("found.md"), content_src)
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let search_res = DocsTool::test_docs_search(
        &server,
        DocsSearchArgs {
            query: "Search".to_string(),
        },
    )
    .await
    .unwrap();
    let json = parse_result(&search_res);
    let returned_path = json["matches"][0]["path"].as_str().unwrap();

    let uri = format!("typstlab://docs/{}", returned_path);

    let res = server
        .test_read_resource_by_uri(&uri)
        .await
        .expect("read_resource failed");

    let content = match &res.contents[0] {
        rmcp::model::ResourceContents::TextResourceContents { text, .. } => text,
        _ => panic!("Text expected"),
    };
    assert_eq!(normalize_content(content), normalize_content(content_src));
}
