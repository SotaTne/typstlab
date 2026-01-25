use super::*;
use crate::context::McpContext;
use std::path::Path;
use tokio::fs;
use typstlab_core::config::consts::search::{MAX_FILE_BYTES, MAX_MATCHES, MAX_SCAN_FILES};
use typstlab_testkit::temp_dir_in_workspace;

#[tokio::test]
async fn test_rules_browse_hardened() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    fs::create_dir_all(&rules_dir).await.unwrap();

    // path フィールドが返ること
    fs::write(rules_dir.join("foo.md"), "body").await.unwrap();
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
    let text = res.content[0].as_text().unwrap();
    assert!(
        text.text.contains("\"path\":\"rules/foo.md\""),
        "rules_browse should include relative path"
    );

    // Test path traversal protection
    #[cfg(unix)]
    {
        // Should fail: path with ..
        let res = RulesTool::rules_browse(
            &server,
            RulesBrowseArgs {
                path: "rules/../papers".to_string(),
            },
        )
        .await;
        assert!(res.is_err());
        assert!(res.unwrap_err().message.contains("cannot contain .."));
    }
}

#[tokio::test]
async fn test_rules_get_and_list() {
    // Note: rules_get is deprecated as a public tool (DESIGN.md 5.10.1)
    // This test uses the internal function for backward compatibility testing
    // TODO: Migrate to read_resource (typstlab://rules/*) based testing
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    fs::write(rules_dir.join("a.md"), "rule a").await.unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // Test list
    let res = RulesTool::rules_list(
        &server,
        RulesListArgs {
            paper_id: None,
            include_root: true,
        },
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().unwrap();
    assert!(text.text.contains("rules/a.md"));

    // Test get (internal use only)
    let res = RulesTool::rules_get(
        &server,
        RulesGetArgs {
            path: "rules/a.md".to_string(),
        },
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().unwrap();
    assert_eq!(text.text, "rule a");
}

#[tokio::test]
async fn test_rules_page() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    fs::write(rules_dir.join("p.md"), "l1\nl2\nl3\nl4")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_page(
        &server,
        RulesPageArgs {
            path: "rules/p.md".to_string(),
            offset: Some(1),
            limit: Some(2),
        },
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().unwrap();
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    assert_eq!(json["content"], "l2\nl3");
    assert_eq!(json["total"], 4);
}

#[tokio::test]
async fn test_rules_search_includes_excerpt() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    fs::write(rules_dir.join("search.md"), "l1\nl2\nmatch here\nl4\nl5")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_search(
        &server,
        RulesSearchArgs {
            query: "match".to_string(),
            paper_id: None,
            include_root: true,
        },
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().unwrap();
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    let excerpt = json["matches"][0]["excerpt"].as_str().unwrap();
    assert_eq!(excerpt, "l1\nl2\nmatch here\nl4\nl5");
}

#[tokio::test]
async fn test_rules_get_rejects_non_markdown() {
    // Note: rules_get is deprecated as a public tool (DESIGN.md 5.10.1)
    // This test uses the internal function for backward compatibility testing
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    fs::write(rules_dir.join("secret.txt"), "nope")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_get(
        &server,
        RulesGetArgs {
            path: "rules/secret.txt".into(),
        },
    )
    .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_rules_get_missing_returns_not_found_code() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_get(
        &server,
        RulesGetArgs {
            path: "rules/missing.md".into(),
        },
    )
    .await;

    assert!(res.is_err());
    let err = res.unwrap_err();
    // resolve_rules_path skips check_entry_safety for non-existent paths
    assert_eq!(err.code, rmcp::model::ErrorCode(-32002)); // NOT_FOUND
    assert_eq!(err.data.unwrap()["code"], crate::errors::NOT_FOUND);
}

#[tokio::test]
async fn test_rules_get_non_markdown_returns_invalid_input_code() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    fs::write(rules_dir.join("secret.txt"), "nope")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_get(
        &server,
        RulesGetArgs {
            path: "rules/secret.txt".into(),
        },
    )
    .await;

    assert!(res.is_err());
    let err = res.unwrap_err();
    assert_eq!(err.code, rmcp::model::ErrorCode(-32602)); // INVALID_INPUT
    assert_eq!(err.data.unwrap()["code"], crate::errors::INVALID_INPUT);
}

#[tokio::test]
async fn test_rules_page_non_markdown_returns_invalid_input_code() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    fs::write(rules_dir.join("secret.txt"), "line")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_page(
        &server,
        RulesPageArgs {
            path: "rules/secret.txt".into(),
            offset: None,
            limit: None,
        },
    )
    .await;

    assert!(res.is_err());
    let err = res.unwrap_err();
    assert_eq!(err.code, rmcp::model::ErrorCode(-32602)); // INVALID_INPUT
    assert_eq!(err.data.unwrap()["code"], crate::errors::INVALID_INPUT);
}

#[tokio::test]
async fn test_rules_page_rejects_non_markdown() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    fs::write(rules_dir.join("secret.txt"), "line")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_page(
        &server,
        RulesPageArgs {
            path: "rules/secret.txt".into(),
            offset: None,
            limit: None,
        },
    )
    .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_rules_browse_rejects_file_path_with_invalid_input() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    fs::write(rules_dir.join("foo.md"), "body").await.unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_browse(
        &server,
        RulesBrowseArgs {
            path: "rules/foo.md".to_string(),
        },
    )
    .await;

    assert!(res.is_err());
    let err = res.unwrap_err();
    assert_eq!(err.code, rmcp::model::ErrorCode(-32602)); // INVALID_INPUT
    assert_eq!(err.data.unwrap()["code"], crate::errors::INVALID_INPUT);
}

#[tokio::test]
async fn test_rules_search_stops_after_file_scan_limit() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    fs::create_dir_all(&rules_dir).await.unwrap();

    // Create many files; put the only match after the scan limit.
    for i in 0..(MAX_SCAN_FILES + 10) {
        let name = format!("f{:04}.md", i);
        let content = if i == MAX_SCAN_FILES + 5 {
            "target line"
        } else {
            "noise"
        };
        fs::write(rules_dir.join(name), content).await.unwrap();
    }

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_search(
        &server,
        RulesSearchArgs {
            query: "target".to_string(),
            paper_id: None,
            include_root: true,
        },
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().unwrap();
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    assert!(json["matches"].as_array().unwrap().is_empty());
    assert_eq!(json["truncated"].as_bool().unwrap(), true);
}

#[tokio::test]
async fn test_rules_search_truncates_at_max_matches_without_clearing() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    fs::create_dir_all(&rules_dir).await.unwrap();

    for i in 0..(MAX_MATCHES + 10) {
        let name = format!("match_{i}.md");
        fs::write(rules_dir.join(name), "needle").await.unwrap();
    }

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_search(
        &server,
        RulesSearchArgs {
            query: "needle".to_string(),
            paper_id: None,
            include_root: true,
        },
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().unwrap();
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();

    assert_eq!(json["truncated"].as_bool().unwrap(), true);
    let matches_len = json["matches"].as_array().unwrap().len();
    assert_eq!(
        matches_len, MAX_MATCHES,
        "matches should be trimmed to MAX_MATCHES, not cleared"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn test_rules_search_skips_symlink_outside_root() {
    use std::os::unix::fs::symlink;

    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    fs::create_dir_all(&rules_dir).await.unwrap();

    let outside_temp = temp_dir_in_workspace();
    let outside_dir = outside_temp.path().join("outside");
    fs::create_dir_all(&outside_dir).await.unwrap();
    let outside_file = outside_dir.join("secret.md");
    fs::write(&outside_file, "leak me").await.unwrap();

    let link_path = rules_dir.join("link.md");
    symlink(&outside_file, &link_path).unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_search(
        &server,
        RulesSearchArgs {
            query: "leak".to_string(),
            paper_id: None,
            include_root: true,
        },
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().unwrap();
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    let matches = json["matches"].as_array().unwrap();
    assert!(matches.is_empty());
}

#[tokio::test]
async fn test_rules_get_rejects_large_file() {
    // Note: rules_get is deprecated as a public tool (DESIGN.md 5.10.1)
    // This test uses the internal function for backward compatibility testing
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    let large = vec![b'a'; (MAX_FILE_BYTES + 1) as usize];
    fs::write(rules_dir.join("big.md"), large).await.unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_get(
        &server,
        RulesGetArgs {
            path: "rules/big.md".into(),
        },
    )
    .await;
    assert!(res.is_err(), "should reject oversized rule file");
}

#[tokio::test]
async fn test_rules_page_rejects_large_file() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    let large = vec![b'a'; (MAX_FILE_BYTES + 1) as usize];
    fs::write(rules_dir.join("big.md"), large).await.unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_page(
        &server,
        RulesPageArgs {
            path: "rules/big.md".into(),
            offset: None,
            limit: None,
        },
    )
    .await;
    assert!(res.is_err(), "should reject oversized rule file");
}

#[tokio::test]
async fn test_rules_list_truncates_and_signals() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    for i in 0..(MAX_SCAN_FILES + 10) {
        let name = format!("f{i:04}.md");
        fs::write(rules_dir.join(name), "body").await.unwrap();
    }

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
    let text = res.content[0].as_text().unwrap();
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    assert!(json["truncated"].as_bool().unwrap());
    assert!(json["files"].as_array().unwrap().is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn test_rules_list_skips_symlink_outside_root() {
    use std::os::unix::fs::symlink;

    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    fs::create_dir_all(&rules_dir).await.unwrap();

    let outside_temp = temp_dir_in_workspace();
    let outside_dir = outside_temp.path().join("outside");
    fs::create_dir_all(&outside_dir).await.unwrap();
    let outside_file = outside_dir.join("secret.md");
    fs::write(&outside_file, "leak me").await.unwrap();

    let link_path = rules_dir.join("link.md");
    symlink(&outside_file, &link_path).unwrap();

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
    let text = res.content[0].as_text().unwrap();
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    let files = json["files"].as_array().unwrap();
    assert!(files.is_empty());
}

#[tokio::test]
async fn test_resolve_rules_path_rejects_rooted_path() {
    let temp = temp_dir_in_workspace();
    let result = resolve_rules_path(temp.path(), Path::new("/tmp")).await;
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.message.contains("absolute or rooted"));
    }
}

#[tokio::test]
async fn test_rules_search_rejects_invalid_paper_id_traversal() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_search(
        &server,
        RulesSearchArgs {
            query: "test".to_string(),
            paper_id: Some("../etc".to_string()),
            include_root: false,
        },
    )
    .await;

    assert!(res.is_err(), "Should reject paper_id with parent traversal");
    let err = res.unwrap_err();
    assert!(
        err.message.contains("PAPER_INVALID_ID"),
        "Error should contain PAPER_INVALID_ID code"
    );
}

#[tokio::test]
async fn test_rules_search_rejects_invalid_paper_id_absolute() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_search(
        &server,
        RulesSearchArgs {
            query: "test".to_string(),
            paper_id: Some("/tmp".to_string()),
            include_root: false,
        },
    )
    .await;

    assert!(res.is_err(), "Should reject absolute paper_id");
    let err = res.unwrap_err();
    assert!(
        err.message.contains("PAPER_INVALID_ID"),
        "Error should contain PAPER_INVALID_ID code"
    );
}

#[tokio::test]
async fn test_rules_search_rejects_invalid_paper_id_multiple_components() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_search(
        &server,
        RulesSearchArgs {
            query: "test".to_string(),
            paper_id: Some("foo/bar".to_string()),
            include_root: false,
        },
    )
    .await;

    assert!(
        res.is_err(),
        "Should reject paper_id with multiple components"
    );
    let err = res.unwrap_err();
    assert!(
        err.message.contains("PAPER_INVALID_ID"),
        "Error should contain PAPER_INVALID_ID code"
    );
}

#[tokio::test]
async fn test_rules_list_rejects_invalid_paper_id_traversal() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_list(
        &server,
        RulesListArgs {
            paper_id: Some("../etc".to_string()),
            include_root: false,
        },
    )
    .await;

    assert!(res.is_err(), "Should reject paper_id with parent traversal");
    let err = res.unwrap_err();
    assert!(
        err.message.contains("PAPER_INVALID_ID"),
        "Error should contain PAPER_INVALID_ID code"
    );
}

#[tokio::test]
async fn test_rules_list_rejects_invalid_paper_id_absolute() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_list(
        &server,
        RulesListArgs {
            paper_id: Some("/tmp".to_string()),
            include_root: false,
        },
    )
    .await;

    assert!(res.is_err(), "Should reject absolute paper_id");
    let err = res.unwrap_err();
    assert!(
        err.message.contains("PAPER_INVALID_ID"),
        "Error should contain PAPER_INVALID_ID code"
    );
}

#[tokio::test]
async fn test_rules_list_rejects_invalid_paper_id_multiple_components() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_list(
        &server,
        RulesListArgs {
            paper_id: Some("foo/bar".to_string()),
            include_root: false,
        },
    )
    .await;

    assert!(
        res.is_err(),
        "Should reject paper_id with multiple components"
    );
    let err = res.unwrap_err();
    assert!(
        err.message.contains("PAPER_INVALID_ID"),
        "Error should contain PAPER_INVALID_ID code"
    );
}
#[tokio::test]
async fn test_rules_search_rejects_empty_query() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_search(
        &server,
        RulesSearchArgs {
            query: "".to_string(),
            paper_id: None,
            include_root: true,
        },
    )
    .await;

    assert!(res.is_err(), "Should reject empty query");
    let err = res.unwrap_err();
    assert!(
        err.message.contains("empty or whitespace-only"),
        "Error should mention empty query"
    );
}

#[tokio::test]
async fn test_rules_search_rejects_whitespace_only_query() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = RulesTool::rules_search(
        &server,
        RulesSearchArgs {
            query: "   \t\n  ".to_string(),
            paper_id: None,
            include_root: true,
        },
    )
    .await;

    assert!(res.is_err(), "Should reject whitespace-only query");
    let err = res.unwrap_err();
    assert!(
        err.message.contains("empty or whitespace-only"),
        "Error should mention whitespace-only query"
    );
}

#[tokio::test]
async fn test_rules_search_rejects_too_long_query() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let long_query = "a".repeat(1001);
    let res = RulesTool::rules_search(
        &server,
        RulesSearchArgs {
            query: long_query,
            paper_id: None,
            include_root: true,
        },
    )
    .await;

    assert!(res.is_err(), "Should reject too long query");
    let err = res.unwrap_err();
    assert!(
        err.message.contains("too long"),
        "Error should mention query length"
    );
}
