//! Unit tests for github module

use typstlab_typst::github::{
    add_path_segments, build_default_client, github_api_base_url, github_base_url,
};

#[test]
fn test_build_client() {
    let client = build_default_client().unwrap();
    // Verify client is constructed successfully
    // The client should be ready to use for HTTP requests
    assert!(std::mem::size_of_val(&client) > 0);
}

#[test]
fn test_github_base_url() {
    let url = github_base_url().unwrap();
    assert_eq!(url.as_str(), "https://github.com/");
}

#[test]
fn test_github_api_base_url() {
    let url = github_api_base_url().unwrap();
    assert_eq!(url.as_str(), "https://api.github.com/");
}

#[test]
fn test_add_path_segments() {
    let mut url = github_base_url().unwrap();
    add_path_segments(&mut url, &["typst", "typst", "archive"]).unwrap();
    assert_eq!(url.as_str(), "https://github.com/typst/typst/archive");
}

#[test]
fn test_add_path_segments_with_special_chars() {
    let mut url = github_base_url().unwrap();
    // path_segments_mut should automatically URL-encode special characters
    add_path_segments(&mut url, &["typst", "typst", "file with spaces"]).unwrap();
    assert!(url.as_str().contains("file%20with%20spaces"));
}

#[test]
fn test_add_path_segments_prevents_injection() {
    let mut url = github_base_url().unwrap();
    // Malicious path segments should be safely encoded
    add_path_segments(&mut url, &["typst", "../../../etc/passwd"]).unwrap();
    // The ../ should be URL-encoded, preventing path traversal
    assert!(url.as_str().contains("%2E%2E%2F") || url.as_str().contains("..%2F"));
}
