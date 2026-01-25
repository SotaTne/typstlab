use tokio_util::sync::CancellationToken;
use typstlab_mcp::handlers::common::ops;
use typstlab_testkit::temp_dir_in_workspace;

#[test]
fn test_symlink_safety_check() {
    let root_temp = temp_dir_in_workspace();
    let root = root_temp.path();

    let secret_temp = temp_dir_in_workspace();
    let secret_file = secret_temp.path().join("secret.txt");
    std::fs::write(&secret_file, "secret data").unwrap();

    let symlink_path = root.join("link_to_secret");

    #[cfg(unix)]
    std::os::unix::fs::symlink(&secret_file, &symlink_path).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&secret_file, &symlink_path).unwrap();

    // check_entry_safety should fail for symlink
    let result = ops::check_entry_safety(&symlink_path, root);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, rmcp::model::ErrorCode(-32001)); // PATH_ESCAPE
    assert!(err.message.contains("Symlinks are not allowed"));
}

#[tokio::test]
async fn test_browse_symlink_skip() {
    let root_temp = temp_dir_in_workspace();
    let root = root_temp.path();

    // Create legit file
    std::fs::write(root.join("safe.md"), "safe").unwrap();

    // Create symlink
    let secret_temp = temp_dir_in_workspace();
    let secret = secret_temp.path().join("secret.md");
    std::fs::write(&secret, "secret").unwrap();
    let link = root.join("unsafe.md");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&secret, &link).unwrap();

    let root_path = root.to_path_buf(); // Clone path before blocking
    let token = CancellationToken::new();

    // Run blocking browse
    let res = tokio::task::spawn_blocking(move || {
        ops::browse_dir_sync(
            &root_path,
            &root_path,
            None,
            &["md".to_string()],
            1000,
            token,
        )
    })
    .await
    .unwrap()
    .unwrap();

    // Should only contain "safe.md"
    assert_eq!(res.items.len(), 1);
    assert_eq!(res.items[0].name, "safe.md");
}
