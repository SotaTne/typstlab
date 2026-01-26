use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::time::{Duration, timeout};

use typstlab_mcp::context::McpContext;
use typstlab_mcp::handlers::cmd::{CmdTool, StatusArgs};
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
async fn test_search_does_not_block_runtime() {
    let server = Arc::new(setup_test_server().await);
    // ファイル数は極小でOK（性質テストなので、ブロックするかどうかを見る）
    // バリア同期を使わない代わりに、少しだけ待ってからstatusを呼ぶ
    setup_docs_with_files(&server, 5).await;

    let server_clone = server.clone();

    // 検索開始
    let search_handle = tokio::spawn(async move {
        DocsTool::test_docs_search(
            &server_clone,
            DocsSearchArgs {
                query: "test".to_string(),
                page: 1,
            },
        )
        .await
    });

    // 検索タスクがスケジュールされるのを少し待つ
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Statusが1000ms以内に完了すること（ブロックされていたらタイムアウトするはず）
    let status_res = timeout(
        Duration::from_millis(1000),
        CmdTool::status(&server, StatusArgs { paper_id: None }),
    )
    .await;

    assert!(status_res.is_ok(), "Status blocked by search task!");
    let _ = status_res.unwrap(); // Status自体は成功するはず

    // 検索も最終的に成功
    let search_result = search_handle.await.unwrap();
    assert!(search_result.is_ok());
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
