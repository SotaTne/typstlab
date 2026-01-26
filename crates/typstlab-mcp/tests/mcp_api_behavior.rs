//! DESIGN.md 5.10.3 の仕様準拠テスト
//!
//! 仕様: Rules/Docsツールの基本API仕様（入力・出力スキーマ、基本挙動）
//! 根拠: DESIGN.md 5.10.3 "Provided MCP Tools"

use serde_json::Value;
use typstlab_mcp::context::McpContext;
use typstlab_mcp::handlers::docs::{DocsBrowseArgs, DocsSearchArgs, DocsTool};
use typstlab_mcp::handlers::rules::{RulesBrowseArgs, RulesListArgs, RulesSearchArgs, RulesTool};
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;

// Helper to parse content result
fn parse_result(result: &rmcp::model::CallToolResult) -> Value {
    let text = result.content[0].as_text().unwrap();
    serde_json::from_str(&text.text).unwrap()
}

#[tokio::test]
async fn test_rules_browse_normal_success() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    tokio::fs::create_dir_all(&rules_dir).await.unwrap();
    tokio::fs::write(rules_dir.join("guidelines.md"), "# Guidelines")
        .await
        .unwrap();

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
    let items = json["items"].as_array().unwrap();

    assert!(items.iter().any(|item| item["name"] == "guidelines.md"
        && item["type"] == "file"
        && item["path"] == "rules/guidelines.md"));
}

#[tokio::test]
async fn test_rules_browse_nonexistent_returns_missing_true() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // rules directory doesn't exist
    let res = RulesTool::rules_browse(
        &server,
        RulesBrowseArgs {
            path: "rules".to_string(),
        },
    )
    .await
    .unwrap();

    let json = parse_result(&res);
    assert!(json["items"].as_array().unwrap().is_empty());
    assert_eq!(json["missing"], true);
}

#[tokio::test]
async fn test_rules_search_normal_success() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    tokio::fs::create_dir_all(&rules_dir).await.unwrap();
    tokio::fs::write(rules_dir.join("citation.md"), "Use APA format")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_search(
        &server,
        RulesSearchArgs {
            query: "APA".to_string(),
            paper_id: None,
            include_root: true,
            page: 1,
        },
    )
    .await
    .unwrap();

    let json = parse_result(&res);
    let matches = json["matches"].as_array().unwrap();
    assert!(!matches.is_empty());
    assert!(matches[0]["preview"].as_str().unwrap().contains("APA"));
}

#[tokio::test]
async fn test_rules_search_nonexistent_returns_missing_true() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_search(
        &server,
        RulesSearchArgs {
            query: "APA".to_string(),
            paper_id: None,
            include_root: true,
            page: 1,
        },
    )
    .await
    .unwrap();

    let json = parse_result(&res);
    assert!(json["matches"].as_array().unwrap().is_empty());
    assert_eq!(json["missing"], true);
}

#[tokio::test]
async fn test_rules_list_normal_success() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    tokio::fs::create_dir_all(&rules_dir).await.unwrap();
    tokio::fs::write(rules_dir.join("rule1.md"), "Rule 1")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_list(
        &server,
        RulesListArgs {
            paper_id: None,
            include_root: true,
        },
    )
    .await
    .unwrap();

    let json = parse_result(&res);
    let items = json["files"].as_array().unwrap();
    assert!(!items.is_empty());
}

#[tokio::test]
async fn test_docs_browse_normal_success() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();
    tokio::fs::write(docs_dir.join("intro.md"), "# Introduction")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::test_docs_browse(
        &server,
        DocsBrowseArgs {
            path: None, // root
        },
    )
    .await
    .unwrap();

    let json = parse_result(&res);
    assert!(!json["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_docs_search_normal_success() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();
    tokio::fs::write(docs_dir.join("math.md"), "Equation logic")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::test_docs_search(
        &server,
        DocsSearchArgs {
            query: "logic".to_string(),
            page: 1,
        },
    )
    .await
    .unwrap();

    let json = parse_result(&res);
    assert!(!json["matches"].as_array().unwrap().is_empty());
}
