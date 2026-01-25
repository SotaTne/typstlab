use serde_json::json;
use tokio_util::sync::CancellationToken;
use typstlab_mcp::handlers::common::{ops, types::SearchConfig};
use typstlab_testkit::temp_dir_in_workspace;

#[test]
fn test_search_mapper_logic() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();

    // Create a file
    let file = root.join("test.md");
    std::fs::write(&file, "line1\nline2\nline3").unwrap();

    let config = SearchConfig::new(10, 10, vec!["md".to_string()]);
    let token = CancellationToken::new();

    // Execute search with a mapper
    let result = ops::search_dir_sync(
        root,
        root, // project_root
        &config,
        token,
        |path, content| {
            assert!(path.ends_with("test.md"));
            assert_eq!(content, "line1\nline2\nline3");

            // Return 2 matches
            Some(vec![json!({"line": 1}), json!({"line": 3})])
        },
    )
    .unwrap();

    assert_eq!(result.matches.len(), 2);
    assert_eq!(result.matches[0]["line"], 1);
    assert_eq!(result.matches[1]["line"], 3);
    assert!(result.scanned_files >= 1);
}

#[test]
fn test_search_limit_matches() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();
    std::fs::write(root.join("test.md"), "content").unwrap();

    // Max matches = 1
    let config = SearchConfig::new(10, 1, vec!["md".to_string()]);
    let token = CancellationToken::new();

    let result = ops::search_dir_sync(
        root,
        root, // project_root
        &config,
        token,
        |_, _| {
            // Return 2 matches
            Some(vec![json!({"id": 1}), json!({"id": 2})])
        },
    )
    .unwrap();

    // Should be truncated to 1
    assert_eq!(result.matches.len(), 1);
    assert!(result.truncated);
}

#[test]
fn test_browse_file_returns_missing() {
    let temp = temp_dir_in_workspace();
    let root = temp.path();
    let file = root.join("file.md");
    std::fs::write(&file, "content").unwrap();

    // Pass file path as root (simulate accidental file path passed)
    // In browse_dir_sync(root=file, project_root=root, ...)
    let res = ops::browse_dir_sync(
        &file,
        root,
        None,
        &["md".to_string()],
        1000,
        CancellationToken::new(),
    )
    .unwrap();

    // Should be missing: true (or empty items), NOT error
    assert!(res.missing);
    assert!(res.items.is_empty());
}
