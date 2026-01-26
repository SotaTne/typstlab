//! DESIGN.md 5.10.9 の仕様準拠テスト
//!
//! 仕様: 制限値と挙動（MAX_SCAN_FILES, MAX_MATCHES, MAX_FILE_BYTES）
//! 根拠: DESIGN.md 5.10.9 "制限値と挙動"

use serde_json::Value;
use typstlab_mcp::context::McpContext;
use typstlab_mcp::handlers::docs::{DocsSearchArgs, DocsTool};
// use typstlab_mcp::handlers::rules::{RulesBrowseArgs, RulesTool};
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;
// use rmcp::ServerHandler;

// Constants from spec
// Constants from spec
use typstlab_core::config::consts::search::MAX_SCAN_FILES;
const MAX_MATCHES: usize = 50;
const MAX_FILE_BYTES: usize = 1024 * 1024; // 1 MiB

fn parse_result(result: &rmcp::model::CallToolResult) -> Value {
    let text = result.content[0].as_text().unwrap();
    serde_json::from_str(&text.text).unwrap()
}

fn get_error_code(err: &rmcp::ErrorData) -> Option<String> {
    err.data
        .as_ref()
        .and_then(|v| v.get("code"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
}

#[tokio::test]
async fn test_max_scan_files_truncation_returns_empty_matches() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();

    // Create 51 files (MAX_SCAN_FILES + 1)
    // Only the last one matches query to verify full scan stop
    for i in 0..=MAX_SCAN_FILES {
        let content = if i == MAX_SCAN_FILES {
            "match_query"
        } else {
            "other"
        };
        tokio::fs::write(docs_dir.join(format!("file_{:04}.md", i)), content)
            .await
            .unwrap();
    }

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::test_docs_search(
        &server,
        DocsSearchArgs {
            query: "match_query".to_string(),
            page: 1,
        },
    )
    .await
    .unwrap();

    let json = parse_result(&res);
    assert_eq!(json["truncated"], true);
    assert_eq!(
        json["matches"].as_array().unwrap().len(),
        0,
        "Should handle partial scan by emptying results"
    );
}

#[tokio::test]
async fn test_max_matches_truncation_caps_results() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();

    // Create files that produce total > MAX_MATCHES
    // Each file has 1 match, create 55 files
    // Create 20 files, each having 10 matches (capped to 3 per file).
    // Total matches = 20 * 3 = 60 > MAX_MATCHES (50).
    // Files 20 < MAX_SCAN_FILES (50), so file limit not hit.
    for i in 0..20 {
        let content = "match_query\n".repeat(10);
        tokio::fs::write(docs_dir.join(format!("match_{:03}.md", i)), content)
            .await
            .unwrap();
    }

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::test_docs_search(
        &server,
        DocsSearchArgs {
            query: "match_query".to_string(),
            page: 1,
        },
    )
    .await
    .unwrap();

    let json = parse_result(&res);
    assert_eq!(json["truncated"], true);
    assert_eq!(json["matches"].as_array().unwrap().len(), MAX_MATCHES);
}

#[tokio::test]
async fn test_max_file_bytes_exceeded_returns_file_too_large() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    tokio::fs::create_dir_all(&rules_dir).await.unwrap();

    // Create file slightly larger than 1MB
    let content = vec![b'a'; MAX_FILE_BYTES + 1];
    tokio::fs::write(rules_dir.join("large.md"), content)
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // Test with read_resource
    let res = server
        .test_read_resource_by_uri("typstlab://rules/rules/large.md")
        .await;

    assert!(res.is_err());
    let err = res.unwrap_err();
    let code = get_error_code(&err).expect("data.code must exist");
    assert_eq!(code, "FILE_TOO_LARGE");
}
