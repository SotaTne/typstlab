//! 厳密なエラーコード検証テスト（v0.2以降）
//!
//! このテストは新しい標準エラーコードのみを受け入れます。
//! 旧コード（PROJECT_PATH_ESCAPE等）は許容されません。

use typstlab_mcp::context::McpContext;
use typstlab_mcp::handlers::docs::DocsTool;
use typstlab_mcp::handlers::rules::RulesTool;
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;

/// ErrorDataからエラーコードを取得するヘルパー関数
fn get_error_code(err: &rmcp::ErrorData) -> Option<String> {
    err.data
        .as_ref()
        .and_then(|v| v.get("code"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
}

#[tokio::test]
async fn test_path_traversal_returns_path_escape() {
    // パストラバーサルでPATH_ESCAPEのみを返すこと（旧コード不可）
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_browse(
        &server,
        typstlab_mcp::handlers::rules::RulesBrowseArgs {
            path: "rules/../../../etc/passwd".to_string(),
        },
    )
    .await;

    assert!(res.is_err(), "Path traversal should return error");
    let err = res.unwrap_err();
    let code = get_error_code(&err).expect("Error must have a code");

    assert_eq!(
        code, "PATH_ESCAPE",
        "Must use standard code PATH_ESCAPE for security violations"
    );
}

#[tokio::test]
async fn test_absolute_path_returns_path_escape() {
    // 絶対パスでPATH_ESCAPEを返すこと
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_browse(
        &server,
        typstlab_mcp::handlers::rules::RulesBrowseArgs {
            path: "/etc/passwd".to_string(),
        },
    )
    .await;

    assert!(res.is_err(), "Absolute path should return error");
    let err = res.unwrap_err();
    let code = get_error_code(&err).expect("Error must have a code");

    assert_eq!(
        code, "PATH_ESCAPE",
        "Must use PATH_ESCAPE for absolute paths"
    );
}

#[tokio::test]
async fn test_empty_query_returns_invalid_input() {
    // 空クエリでINVALID_INPUTを返すこと
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_search(
        &server,
        typstlab_mcp::handlers::rules::RulesSearchArgs {
            query: "   ".to_string(), // 空白のみ
            paper_id: None,
            include_root: true,
        },
    )
    .await;

    assert!(res.is_err(), "Empty query should return error");
    let err = res.unwrap_err();
    let code = get_error_code(&err).expect("Error must have a code");

    assert_eq!(
        code, "INVALID_INPUT",
        "Must use INVALID_INPUT for empty query"
    );
}

#[tokio::test]
async fn test_whitespace_query_returns_invalid_input() {
    // 空白のみクエリでINVALID_INPUTを返すこと
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::test_docs_search(
        &server,
        typstlab_mcp::handlers::docs::DocsSearchArgs {
            query: "\t\n  ".to_string(),
        },
    )
    .await;

    assert!(res.is_err(), "Whitespace-only query should return error");
    let err = res.unwrap_err();
    let code = get_error_code(&err).expect("Error must have a code");

    assert_eq!(
        code, "INVALID_INPUT",
        "Must use INVALID_INPUT for whitespace query"
    );
}

#[tokio::test]
async fn test_no_legacy_codes_allowed() {
    // 旧コードが絶対に返らないことを確認
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // 複数のエラーケースをテスト
    let test_cases = vec![
        (
            "Path traversal",
            RulesTool::rules_browse(
                &server,
                typstlab_mcp::handlers::rules::RulesBrowseArgs {
                    path: "../../../etc".to_string(),
                },
            )
            .await,
        ),
        (
            "Absolute path",
            RulesTool::rules_browse(
                &server,
                typstlab_mcp::handlers::rules::RulesBrowseArgs {
                    path: "/tmp/test".to_string(),
                },
            )
            .await,
        ),
    ];

    for (name, result) in test_cases {
        if let Err(err) = result
            && let Some(code) = get_error_code(&err)
        {
            // 旧コードが返らないことを確認
            assert_ne!(
                code, "PROJECT_PATH_ESCAPE",
                "{}: Must not use legacy code PROJECT_PATH_ESCAPE",
                name
            );
            assert_ne!(
                code, "PROJECT_NOT_FOUND",
                "{}: Must not use legacy code PROJECT_NOT_FOUND",
                name
            );
        }
    }
}

#[tokio::test]
async fn test_error_messages_are_descriptive() {
    // エラーメッセージが説明的であること
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_browse(
        &server,
        typstlab_mcp::handlers::rules::RulesBrowseArgs {
            path: "rules/../../../etc".to_string(),
        },
    )
    .await;

    assert!(res.is_err());
    let err = res.unwrap_err();

    // メッセージに説明的な文言が含まれること
    assert!(
        err.message.contains("..")
            || err.message.contains("path")
            || err.message.contains("Path")
            || err.message.contains("outside"),
        "Error message should be descriptive, got: {}",
        err.message
    );
}
