use std::sync::Arc;
use tokio::sync::oneshot;

use typstlab_mcp::context::McpContext;

use typstlab_mcp::handlers::docs::{DocsSearchArgs, DocsTool};
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;

/// テスト用サーバーセットアップ
async fn setup_test_server() -> TypstlabServer {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    TypstlabServer::new(ctx, false)
}

/// テスト用ドキュメントを生成するヘルパー
async fn setup_docs_with_files(server: &TypstlabServer, count: usize) {
    let docs_dir = server.context.project_root.join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();

    for i in 0..count {
        let file_path = docs_dir.join(format!("file_{}.md", i));
        tokio::fs::write(
            file_path,
            format!(
                "This is test file number {}.\nContent with keyword test.",
                i
            ),
        )
        .await
        .unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_runtime_remains_responsive() {
    // yield_nowが機能することを確認
    let (tx, rx) = oneshot::channel();
    let server = Arc::new(setup_test_server().await);
    setup_docs_with_files(&server, 5).await;

    let server_clone = server.clone();
    let _search_handle = tokio::spawn(async move {
        tx.send(()).ok(); // 開始通知
        DocsTool::test_docs_search(
            &server_clone,
            DocsSearchArgs {
                query: "test".to_string(),
                page: 1,
            },
        )
        .await
    });

    // 検索開始まで待機
    rx.await.ok();

    // yield_nowが即座に戻ること（CPUを占有し続けていないこと）
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
}
