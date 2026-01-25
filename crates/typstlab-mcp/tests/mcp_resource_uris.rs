//! DESIGN.md 5.10.8 の仕様準拠テスト
//!
//! 仕様: Rules リソース URI の正規スコープ検証
//! 根拠: DESIGN.md 5.10.8 "Rules リソース URI の正規スコープ"

use typstlab_mcp::context::McpContext;
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;
// use rmcp::ServerHandler;

// Helper function to extract error code data
fn get_error_code(err: &rmcp::ErrorData) -> Option<String> {
    err.data
        .as_ref()
        .and_then(|v| v.get("code"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
}

#[tokio::test]
async fn test_read_resource_rules_uri_valid_success() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    tokio::fs::create_dir_all(&rules_dir).await.unwrap();
    tokio::fs::write(rules_dir.join("guide.md"), "Guide content")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = server
        .test_read_resource_by_uri("typstlab://rules/rules/guide.md")
        .await;

    assert!(res.is_ok());

    // Content check
    let res = res.unwrap();
    let content = &res.contents[0];
    if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = content {
        assert_eq!(text, "Guide content");
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_read_resource_rules_uri_subdir_success() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules/subdir");
    tokio::fs::create_dir_all(&rules_dir).await.unwrap();
    tokio::fs::write(rules_dir.join("nested.md"), "Nested content")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = server
        .test_read_resource_by_uri("typstlab://rules/rules/subdir/nested.md")
        .await;

    assert!(res.is_ok());
}

#[tokio::test]
async fn test_read_resource_rules_paper_uri_valid_success() {
    let temp = temp_dir_in_workspace();
    let paper_rules_dir = temp.path().join("papers/paper1/rules");
    tokio::fs::create_dir_all(&paper_rules_dir).await.unwrap();
    tokio::fs::write(paper_rules_dir.join("cite.md"), "Citation rules")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = server
        .test_read_resource_by_uri("typstlab://rules/papers/paper1/rules/cite.md")
        .await;

    assert!(res.is_ok());
}

#[tokio::test]
async fn test_read_resource_parent_traversal_returns_path_escape() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = server
        .test_read_resource_by_uri("typstlab://rules/../secret.md")
        .await;

    assert!(res.is_err());
    let err = res.unwrap_err();
    let code = get_error_code(&err).expect("data.code must exist");
    assert_eq!(code, "PATH_ESCAPE");
}

#[tokio::test]
async fn test_read_resource_absolute_path_returns_path_escape() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = server
        .test_read_resource_by_uri("typstlab://rules//etc/passwd")
        .await;

    assert!(res.is_err());
    let err = res.unwrap_err();
    let code = get_error_code(&err).expect("data.code must exist");
    assert_eq!(code, "PATH_ESCAPE");
}

#[tokio::test]
async fn test_read_resource_invalid_scope_returns_invalid_input() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = server
        .test_read_resource_by_uri("typstlab://rules/paper1/notes.md")
        .await;

    assert!(res.is_err());
    let err = res.unwrap_err();
    let code = get_error_code(&err).expect("data.code must exist");
    assert_eq!(code, "INVALID_INPUT");
}

#[tokio::test]
async fn test_read_resource_double_prefix_returns_invalid_input() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = server
        .test_read_resource_by_uri("typstlab://rules/papers/paper1/notes.md")
        .await;

    assert!(res.is_err());
    let err = res.unwrap_err();
    let code = get_error_code(&err).expect("data.code must exist");
    assert_eq!(code, "INVALID_INPUT");
}
