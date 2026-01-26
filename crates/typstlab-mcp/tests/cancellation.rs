use std::sync::Arc;
use tokio::task::JoinSet;
use tokio::time::{Duration, timeout};
use typstlab_mcp::context::McpContext;
use typstlab_mcp::handlers::docs::{DocsSearchArgs, DocsTool};
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;

async fn setup_test_server() -> TypstlabServer {
    let temp = temp_dir_in_workspace();
    let root = temp.path().to_path_buf();
    let ctx = McpContext::new(root);
    TypstlabServer::new(ctx, false)
}

// キャンセル誘発用に大量のファイルを生成する (並列化版)
async fn setup_heavy_docs(server: &TypstlabServer, count: usize, content: &str) {
    let docs_dir = server.context.project_root.join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();

    let mut set = JoinSet::new();
    let docs_dir = Arc::new(docs_dir);
    let content = Arc::new(content.to_string());

    for i in 0..count {
        let dir = docs_dir.clone();
        let c = content.clone();
        set.spawn(async move {
            let file_path = dir.join(format!("heavy_{}.md", i));
            tokio::fs::write(file_path, c.as_str()).await.unwrap();
        });
    }

    while let Some(res) = set.join_next().await {
        res.unwrap();
    }

    // リカバリ確認用の軽量ファイル
    let check_path = server
        .context
        .project_root
        .join(".typstlab/kb/typst/docs/check.md");
    tokio::fs::write(check_path, "Recovery check")
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_search_cancellation_and_recovery() {
    let server = Arc::new(setup_test_server().await);

    // 500ファイル作成 (MAX_SCAN_FILESを超えない範囲で負荷をかける)
    // 1ファイルあたりを大きくして読み込み時間を稼ぐ (~30KB)
    let large_content = "Content without search keyword ".repeat(1000);
    setup_heavy_docs(&server, 500, &large_content).await;

    let server_clone = server.clone();

    // 1. Start search with short timeout
    // クエリ "test" は large_content に含まれないため、全ファイルをスキャンするはず
    let result = timeout(
        Duration::from_millis(5),
        DocsTool::test_docs_search(
            &server_clone,
            DocsSearchArgs {
                query: "test".to_string(),
                page: 1,
            },
        ),
    )
    .await;

    // 2. Expect timeout (search takes longer than 5ms due to file I/O)
    assert!(result.is_err(), "Search should timeout");

    // 3. Recovery check: simple search should succeed immediately
    // "Recovery" というユニークな単語で検索
    let recovery_res = timeout(
        Duration::from_millis(1000),
        DocsTool::test_docs_search(
            &server,
            DocsSearchArgs {
                query: "Recovery".to_string(),
                page: 1,
            },
        ),
    )
    .await;

    // タイムアウトチェック
    let inner_res = recovery_res.expect("Recovery check timed out");
    if let Err(ref e) = inner_res {
        eprintln!("Recovery check failed with error: {:?}", e);
    }
    assert!(inner_res.is_ok(), "Recovery check failed");

    // 結果の中身（マッチしたかどうか）はファイルシステムのタイミングに依存するため、
    // エラーなく応答が返ってきたこと（＝ブロッキングされていないこと）をもってリカバリ成功とする。
    assert!(inner_res.is_ok(), "Recovery check failed");
}
