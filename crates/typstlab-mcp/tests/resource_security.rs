use typstlab_mcp::context::McpContext;
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;

#[tokio::test]
async fn test_rules_resource_missing_flag() {
    let temp = temp_dir_in_workspace();
    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = server
        .test_read_resource_by_uri("typstlab://rules")
        .await
        .unwrap();

    let content_json = serde_json::to_value(&res.contents[0]).unwrap();
    let text = content_json["text"]
        .as_str()
        .expect("text field missing")
        .to_string();

    let json: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(json.get("missing").is_some(), "Should have missing field");
    assert!(
        json["missing"].as_bool().unwrap(),
        "Should be missing: true"
    );
    assert!(json["items"].as_array().unwrap().is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn test_docs_resource_symlink_root_rejection() {
    use std::os::unix::fs::symlink;

    let temp = temp_dir_in_workspace();
    let root = temp.path();

    let external_dir = temp_dir_in_workspace();
    let external_docs = external_dir.path().join("docs");
    std::fs::create_dir_all(&external_docs).unwrap();
    std::fs::write(external_docs.join("leak.md"), "secret").unwrap();

    let typst_kb = root.join(".typstlab/kb/typst");
    std::fs::create_dir_all(&typst_kb).unwrap();

    symlink(&external_docs, typst_kb.join("docs")).unwrap();

    let ctx = McpContext::new(root.to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = server.test_read_resource_by_uri("typstlab://docs").await;

    assert!(res.is_err(), "Symlink root should be rejected");
    let err = res.unwrap_err();
    assert_eq!(err.code, rmcp::model::ErrorCode(-32001)); // PATH_ESCAPE);
}

#[cfg(unix)]
#[tokio::test]
async fn test_docs_file_rejects_symlinked_root_outside_project() {
    use std::os::unix::fs::symlink;

    let temp = temp_dir_in_workspace();
    let project_root = temp.path();

    // external docs directory outside project
    let external = temp_dir_in_workspace();
    let external_docs = external.path().join("docs");
    std::fs::create_dir_all(&external_docs).unwrap();
    std::fs::write(external_docs.join("leak.md"), "secret").unwrap();

    // symlink .typstlab/kb/typst/docs -> external_docs
    let kb_root = project_root.join(".typstlab/kb/typst");
    std::fs::create_dir_all(&kb_root).unwrap();
    symlink(&external_docs, kb_root.join("docs")).unwrap();

    let ctx = McpContext::new(project_root.to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    let res = server
        .test_read_resource_by_uri("typstlab://docs/leak.md")
        .await;

    assert!(
        res.is_err(),
        "should reject docs file when docs root is symlink outside project"
    );
    let err = res.unwrap_err();
    assert_eq!(err.code, rmcp::model::ErrorCode(-32001)); // PATH_ESCAPE
}
