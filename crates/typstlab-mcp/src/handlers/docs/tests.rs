use super::{DocsBrowseArgs, DocsSearchArgs, DocsTool, resolve_docs_path};
use crate::context::McpContext;
use crate::server::TypstlabServer;
use std::path::Path;
use tokio::fs;
use tokio_util::sync::CancellationToken;
use typstlab_core::config::consts::search::MAX_SCAN_FILES;
use typstlab_testkit::temp_dir_in_workspace;

#[tokio::test]
async fn test_docs_browse_empty() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::docs_browse(
        &server,
        DocsBrowseArgs { path: None },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let content = &res.content[0];
    let text = content.as_text().expect("Expected text content");
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    assert!(json["missing"].as_bool().unwrap());
    assert!(json["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_docs_browse_with_files() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).await.unwrap();
    fs::write(docs_dir.join("test.md"), "# Test").await.unwrap();
    fs::create_dir(docs_dir.join("subdir")).await.unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::docs_browse(
        &server,
        DocsBrowseArgs { path: None },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().expect("Expected text content");
    assert!(text.text.contains("\"name\":\"test.md\""));
    assert!(text.text.contains("\"type\":\"file\""));
    assert!(text.text.contains("\"name\":\"subdir\""));
    assert!(text.text.contains("\"type\":\"directory\""));
    // Phase 1.5: 存在するパスではmissingフィールドは無い
}

#[tokio::test]
async fn test_docs_search() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).await.unwrap();
    fs::write(docs_dir.join("a.md"), "hello world")
        .await
        .unwrap();
    fs::write(docs_dir.join("b.md"), "rust programming")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::docs_search(
        &server,
        DocsSearchArgs {
            query: "rust".to_string(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().expect("Expected text content");
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    let matches = json["matches"].as_array().unwrap();
    assert!(!matches.is_empty());
    let match_path = matches[0]["path"].as_str().unwrap();
    assert!(
        match_path.ends_with("b.md"),
        "Path '{match_path}' did not end with 'b.md'"
    );
    assert!(matches[0]["line"] == 1);
    let content = matches[0]["content"].as_str().unwrap();
    assert!(content.contains("rust programming"));
    assert!(!text.text.contains("a.md"));
}

#[tokio::test]
async fn test_docs_search_respects_file_scan_limit() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).await.unwrap();

    for i in 0..(MAX_SCAN_FILES + 5) {
        let name = format!("f{i}.md");
        let content = if i == MAX_SCAN_FILES + 2 {
            "needle"
        } else {
            "hay"
        };
        fs::write(docs_dir.join(name), content).await.unwrap();
    }

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::docs_search(
        &server,
        DocsSearchArgs {
            query: "needle".to_string(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().expect("Expected text content");
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    assert!(json["matches"].as_array().unwrap().is_empty());
    assert!(json["truncated"].as_bool().unwrap());
}

#[tokio::test]
async fn test_docs_search_truncation_clears_matches() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).await.unwrap();

    for i in 0..(MAX_SCAN_FILES + 5) {
        let name = format!("f{i:04}.md");
        // No matches; we want truncation triggered by scan limit, not match limit.
        fs::write(docs_dir.join(name), "hay").await.unwrap();
    }

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::docs_search(
        &server,
        DocsSearchArgs {
            query: "needle".to_string(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().unwrap();
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    assert!(json["truncated"].as_bool().unwrap());
    assert!(json["matches"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_docs_browse_missing_returns_json() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::docs_browse(
        &server,
        DocsBrowseArgs { path: None },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().expect("Expected text content");
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    assert!(json["missing"].as_bool().unwrap());
    assert!(json["items"].as_array().unwrap().is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn test_docs_search_skips_symlink_outside_root() {
    use std::os::unix::fs::symlink;

    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).await.unwrap();

    let outside_temp = temp_dir_in_workspace();
    let outside_dir = outside_temp.path().join("outside");
    fs::create_dir_all(&outside_dir).await.unwrap();
    let outside_file = outside_dir.join("leak.md");
    fs::write(&outside_file, "secret payload").await.unwrap();

    let link_path = docs_dir.join("link.md");
    symlink(&outside_file, &link_path).unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::docs_search(
        &server,
        DocsSearchArgs {
            query: "secret".to_string(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().expect("Expected text content");
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    let matches = json["matches"].as_array().unwrap();
    assert!(matches.is_empty());
}

#[tokio::test]
async fn test_resolve_docs_path_rejects_rooted_path() {
    let temp = temp_dir_in_workspace();
    let docs_root = temp.path().join(".typstlab/kb/typst/docs");
    let result = resolve_docs_path(temp.path(), &docs_root, Path::new("/tmp")).await;
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.message.contains("absolute or rooted"));
    }
}

#[tokio::test]
async fn test_docs_search_rejects_empty_query() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::docs_search(
        &server,
        DocsSearchArgs {
            query: "".to_string(),
        },
        CancellationToken::new(),
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
async fn test_docs_search_rejects_whitespace_only_query() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::docs_search(
        &server,
        DocsSearchArgs {
            query: "   \t\n  ".to_string(),
        },
        CancellationToken::new(),
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
async fn test_docs_search_rejects_too_long_query() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let long_query = "a".repeat(1001);
    let res = DocsTool::docs_search(
        &server,
        DocsSearchArgs { query: long_query },
        CancellationToken::new(),
    )
    .await;

    assert!(res.is_err(), "Should reject too long query");
    let err = res.unwrap_err();
    assert!(
        err.message.contains("too long"),
        "Error should mention query length"
    );
}

#[tokio::test]
async fn test_docs_browse_rejects_path_traversal() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).await.unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::docs_browse(
        &server,
        DocsBrowseArgs {
            path: Some("../../../etc".to_string()),
        },
        CancellationToken::new(),
    )
    .await;

    assert!(res.is_err(), "Should reject path traversal");
    let err = res.unwrap_err();
    assert!(
        err.message.contains("cannot contain .."),
        "Error should mention path traversal"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn test_docs_browse_skips_symlink_outside_root() {
    use std::os::unix::fs::symlink;

    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).await.unwrap();

    let outside_temp = temp_dir_in_workspace();
    let outside_dir = outside_temp.path().join("outside");
    fs::create_dir_all(&outside_dir).await.unwrap();
    let outside_file = outside_dir.join("secret.md");
    fs::write(&outside_file, "leak me").await.unwrap();

    let link_path = docs_dir.join("link.md");
    symlink(&outside_file, &link_path).unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::docs_browse(
        &server,
        DocsBrowseArgs { path: None },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().expect("Expected text content");
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    let items = json["items"].as_array().unwrap();
    assert!(items.is_empty(), "Should skip symlink outside root");
}

#[tokio::test]
async fn test_resolve_docs_path_rejects_parent_traversal() {
    let temp = temp_dir_in_workspace();
    let docs_root = temp.path().join(".typstlab/kb/typst/docs");
    let result = resolve_docs_path(temp.path(), &docs_root, Path::new("../etc")).await;
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.message.contains("cannot contain .."));
    }
}

#[tokio::test]
async fn test_resolve_docs_path_rejects_multiple_traversal() {
    let temp = temp_dir_in_workspace();
    let docs_root = temp.path().join(".typstlab/kb/typst/docs");
    let result =
        resolve_docs_path(temp.path(), &docs_root, Path::new("foo/bar/../../../etc")).await;
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.message.contains("cannot contain .."));
    }
}

#[tokio::test]
async fn test_docs_browse_returns_files_and_dirs() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).await.unwrap();
    fs::write(docs_dir.join("file.md"), "content")
        .await
        .unwrap();
    fs::create_dir_all(docs_dir.join("subdir")).await.unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::docs_browse(
        &server,
        DocsBrowseArgs { path: None },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().expect("Expected text content");
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    let items = json["items"].as_array().unwrap();

    assert_eq!(items.len(), 2, "Should have file and directory");
    let names: Vec<&str> = items
        .iter()
        .map(|item| item["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"file.md"));
    assert!(names.contains(&"subdir"));
}

#[tokio::test]
async fn test_docs_search_case_insensitive() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).await.unwrap();
    fs::write(docs_dir.join("test.md"), "UPPERCASE content")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::docs_search(
        &server,
        DocsSearchArgs {
            query: "uppercase".to_string(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().expect("Expected text content");
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    let matches = json["matches"].as_array().unwrap();
    assert!(!matches.is_empty(), "Should find case-insensitive match");
}

#[tokio::test]
async fn test_resolve_docs_path_accepts_valid_subpath() {
    let temp = temp_dir_in_workspace();
    let docs_root = temp.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_root).await.unwrap();
    fs::create_dir_all(docs_root.join("subdir")).await.unwrap();
    fs::write(docs_root.join("subdir/file.md"), "content")
        .await
        .unwrap();

    let result = resolve_docs_path(temp.path(), &docs_root, Path::new("subdir/file.md")).await;
    assert!(result.is_ok(), "Should accept valid subpath");
}

#[tokio::test]
async fn test_docs_browse_subdir() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).await.unwrap();
    fs::create_dir_all(docs_dir.join("subdir")).await.unwrap();
    fs::write(docs_dir.join("subdir/nested.md"), "nested content")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::docs_browse(
        &server,
        DocsBrowseArgs {
            path: Some("subdir".to_string()),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().expect("Expected text content");
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    let items = json["items"].as_array().unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"].as_str().unwrap(), "nested.md");
}

#[tokio::test]
async fn test_docs_search_returns_multiple_matches() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).await.unwrap();
    fs::write(docs_dir.join("a.md"), "rust is great\nrust is fast")
        .await
        .unwrap();
    fs::write(docs_dir.join("b.md"), "rust programming")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::docs_search(
        &server,
        DocsSearchArgs {
            query: "rust".to_string(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().expect("Expected text content");
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    let matches = json["matches"].as_array().unwrap();

    // Should have matches from both files
    assert!(matches.len() >= 2, "Should have multiple matches");
}

#[tokio::test]
async fn test_resolve_docs_path_rejects_absolute_path() {
    let temp = temp_dir_in_workspace();
    let docs_root = temp.path().join(".typstlab/kb/typst/docs");
    let result = resolve_docs_path(temp.path(), &docs_root, Path::new("/etc/passwd")).await;
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.message.contains("absolute or rooted"));
    }
}

#[tokio::test]
async fn test_docs_browse_empty_subdir() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).await.unwrap();
    fs::create_dir_all(docs_dir.join("empty")).await.unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = DocsTool::docs_browse(
        &server,
        DocsBrowseArgs {
            path: Some("empty".to_string()),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let text = res.content[0].as_text().expect("Expected text content");
    let json: serde_json::Value = serde_json::from_str(&text.text).unwrap();
    let items = json["items"].as_array().unwrap();

    assert!(items.is_empty(), "Empty directory should have no items");
    // Phase 1.5: 存在するパスではmissingフィールドは無い
}
