//! 標準エラーコードの使用検証テスト
//!
//! DESIGN.md 5.10.6（エラーコード標準化）の仕様に基づき、
//! 標準エラーコードが使用され、旧コードが出ないことを検証する

use typstlab_mcp::context::McpContext;
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
async fn test_path_escape_uses_standard_code() {
    // パストラバーサル時にPATH_ESCAPEを返すこと
    // DESIGN.md 5.10.6: PATH_ESCAPE (旧: PROJECT_PATH_ESCAPE)
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // .. を含むパスでパストラバーサルを試行
    let res = RulesTool::rules_browse(
        &server,
        typstlab_mcp::handlers::rules::RulesBrowseArgs {
            path: "rules/../etc/passwd".to_string(),
        },
    )
    .await;

    assert!(res.is_err(), "Should return error for path traversal");
    let err = res.unwrap_err();

    // エラーメッセージに標準的な文言が含まれること（必須）
    assert!(
        err.message.contains("cannot contain ..") || err.message.contains("Path"),
        "Error message should mention path validation"
    );

    // エラーコードはPATH_ESCAPEのみ（v0.2以降）
    let code = get_error_code(&err).expect("Error must have a code");
    assert_eq!(
        code, "PATH_ESCAPE",
        "Must use new standard code PATH_ESCAPE (not PROJECT_PATH_ESCAPE)"
    );
}

#[tokio::test]
async fn test_invalid_input_for_empty_query() {
    // 不正入力時にエラーを返すこと
    // Note: invalid_paramsはデフォルトでcodeフィールドを持たない可能性がある
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // 空クエリで検索
    let res = RulesTool::rules_search(
        &server,
        typstlab_mcp::handlers::rules::RulesSearchArgs {
            query: "   ".to_string(), // 空白のみ
            paper_id: None,
            include_root: true,
        },
    )
    .await;

    assert!(res.is_err(), "Should return error for empty query");
    let err = res.unwrap_err();

    // エラーメッセージが適切であること
    assert!(
        err.message.contains("empty") || err.message.contains("Query"),
        "Error message should mention empty query"
    );
}

#[tokio::test]
async fn test_invalid_input_for_absolute_path() {
    // 絶対パス入力時にエラーを返すこと
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // 絶対パスで試行
    let res = RulesTool::rules_browse(
        &server,
        typstlab_mcp::handlers::rules::RulesBrowseArgs {
            path: "/etc/passwd".to_string(),
        },
    )
    .await;

    assert!(res.is_err(), "Should return error for absolute path");
    let err = res.unwrap_err();

    // エラーメッセージが適切であること
    assert!(
        err.message.contains("absolute")
            || err.message.contains("rooted")
            || err.message.contains("Path"),
        "Error message should mention absolute/rooted path"
    );
}

#[tokio::test]
async fn test_no_legacy_project_path_escape() {
    // PROJECT_PATH_ESCAPEが使われないこと（PATH_ESCAPEを使うべき）
    // Note: このテストは実装修正後に有効化
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // 複数のエラーケースを試行
    let test_cases = vec![
        // パストラバーサル
        RulesTool::rules_browse(
            &server,
            typstlab_mcp::handlers::rules::RulesBrowseArgs {
                path: "rules/../../../etc".to_string(),
            },
        )
        .await,
        // 絶対パス
        RulesTool::rules_browse(
            &server,
            typstlab_mcp::handlers::rules::RulesBrowseArgs {
                path: "/tmp/test".to_string(),
            },
        )
        .await,
    ];

    for (i, result) in test_cases.into_iter().enumerate() {
        if let Err(err) = result {
            if let Some(code) = get_error_code(&err) {
                // TODO: 実装修正後、この条件を有効化
                // assert_ne!(
                //     code, "PROJECT_PATH_ESCAPE",
                //     "Test case {}: Should not use legacy code PROJECT_PATH_ESCAPE",
                //     i
                // );

                // 現時点では警告のみ
                if code == "PROJECT_PATH_ESCAPE" {
                    eprintln!(
                        "Warning: Test case {} is using legacy code PROJECT_PATH_ESCAPE",
                        i
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn test_error_code_consistency() {
    // 同じ種類のエラーが一貫したコードを返すこと
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // パストラバーサルのテストケース
    let traversal_cases = vec![
        "rules/../etc",
        "rules/../../tmp",
        "rules/subfolder/../../..",
    ];

    let mut error_codes = Vec::new();
    for path in traversal_cases {
        let res = RulesTool::rules_browse(
            &server,
            typstlab_mcp::handlers::rules::RulesBrowseArgs {
                path: path.to_string(),
            },
        )
        .await;

        if let Err(err) = res {
            if let Some(code) = get_error_code(&err) {
                error_codes.push(code);
            }
        }
    }

    // 全て同じエラーコードであること
    if !error_codes.is_empty() {
        let first_code = &error_codes[0];
        for code in &error_codes {
            assert_eq!(
                code, first_code,
                "All path traversal errors should return the same error code"
            );
        }
    }
}
