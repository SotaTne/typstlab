//! MCP Cancellation の仕様準拠テスト
//!
//! 仕様: キャンセル伝播（長時間処理の中断とリソース解放）
//! 根拠: MCP Specification "Cancellation", DESIGN.md

use std::sync::Arc;
use tokio::time::{Duration, timeout};
use typstlab_mcp::context::McpContext;
use typstlab_mcp::handlers::docs::{DocsSearchArgs, DocsTool};
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;
// use rmcp::ServerHandler;

async fn setup_test_server() -> TypstlabServer {
    let temp = temp_dir_in_workspace();
    let root = temp.path().to_path_buf();
    let ctx = McpContext::new(root);
    TypstlabServer::new(ctx, false)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_read_resource_cancellation() {
    let server = Arc::new(setup_test_server().await);
    let rules_dir = server.context.project_root.join("rules");
    tokio::fs::create_dir_all(&rules_dir).await.unwrap();

    // No need for large file anymore as we use explicit token cancellation
    tokio::fs::write(rules_dir.join("normal.md"), "content")
        .await
        .unwrap();

    let uri = "typstlab://rules/rules/normal.md".to_string();

    // 1. Create a token and cancel it immediately
    let token = tokio_util::sync::CancellationToken::new();
    token.cancel();

    // 2. Call read_resource with cancelled token
    let result = server.test_read_resource_with_token(&uri, token).await;

    // 3. Expect RequestCancelled error
    assert!(result.is_err(), "Should fail with cancelled token");
    let err = result.unwrap_err();
    assert_eq!(
        err.code,
        rmcp::model::ErrorCode(-32800),
        "Should return RequestCancelled code (-32800)"
    );

    // 4. Recovery check: read using fresh token
    let recovery_res = server.test_read_resource_by_uri(&uri).await;
    assert!(recovery_res.is_ok(), "Recovery read failed");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_search_cancellation() {
    let server = Arc::new(setup_test_server().await);
    let docs_dir = server.context.project_root.join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();

    // Create many files (e.g. 500)
    for i in 0..1000 {
        tokio::fs::write(docs_dir.join(format!("file_{}.md", i)), "content")
            .await
            .unwrap();
    }

    let server_clone = server.clone();

    // 1. Start search with short timeout
    let result = timeout(
        Duration::from_millis(1),
        DocsTool::test_docs_search(
            &server_clone,
            DocsSearchArgs {
                query: "missing".to_string(),
                page: 1,
            },
        ),
    )
    .await;

    // 2. Expect timeout
    assert!(result.is_err(), "Search should timeout/cancel");

    // 3. Recovery check
    tokio::fs::write(docs_dir.join("check.md"), "check")
        .await
        .unwrap();
    let recovery_res = timeout(
        Duration::from_secs(1),
        DocsTool::test_docs_search(
            &server,
            DocsSearchArgs {
                query: "check".to_string(),
                page: 1,
            },
        ),
    )
    .await;

    assert!(recovery_res.is_ok(), "Recovery search timed out");
    assert!(recovery_res.unwrap().is_ok(), "Recovery search failed");
}
