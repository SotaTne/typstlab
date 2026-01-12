//! Integration tests for docs module and shared GitHub architecture

use typstlab_typst::{docs, github};

#[test]
fn test_url_construction_safety() {
    use docs::build_docs_archive_url;

    // Malicious version should be safely encoded
    let url = build_docs_archive_url("../../../etc/passwd").unwrap();
    // The ../ should be URL-encoded, preventing path traversal
    assert!(
        url.as_str().contains("%2E%2E%2F") || url.as_str().contains("..%2F"),
        "Path traversal characters should be encoded"
    );
    assert!(
        url.as_str().starts_with("https://github.com/typst/typst/"),
        "URL should still point to typst repo"
    );
}

#[test]
fn test_url_special_characters_safety() {
    use docs::build_docs_archive_url;

    // Special characters should be encoded
    let url = build_docs_archive_url("version with spaces").unwrap();
    // Spaces should be percent-encoded
    assert!(
        url.as_str().contains("%20"),
        "Spaces should be percent-encoded as %20"
    );
    // URL should still be valid and point to correct repo
    assert!(
        url.as_str().starts_with("https://github.com/typst/typst/"),
        "URL should point to typst repo"
    );
}

#[test]
fn test_github_module_client_construction() {
    // Verify github module can be used independently
    let client = github::build_default_client();
    assert!(
        client.is_ok(),
        "Should successfully build HTTP client with default settings"
    );
}

#[test]
fn test_github_module_custom_timeout() {
    use std::time::Duration;

    // Verify custom timeout configuration
    let client = github::build_client(Duration::from_secs(60));
    assert!(
        client.is_ok(),
        "Should successfully build HTTP client with custom timeout"
    );
}

#[test]
fn test_github_base_url_construction() {
    let url = github::github_base_url().unwrap();
    assert_eq!(url.as_str(), "https://github.com/");
}

#[test]
fn test_github_api_base_url_construction() {
    let url = github::github_api_base_url().unwrap();
    assert_eq!(url.as_str(), "https://api.github.com/");
}

#[test]
fn test_github_url_path_segments_safety() {
    let mut url = github::github_base_url().unwrap();
    // Add path segments with potential injection attempts
    github::add_path_segments(&mut url, &["typst", "../../../etc", "passwd"]).unwrap();
    // Verify encoding prevents traversal
    let url_str = url.as_str();
    assert!(
        url_str.contains("%2E%2E") || url_str.starts_with("https://github.com/typst/"),
        "Path traversal should be prevented by encoding"
    );
}

#[test]
fn test_docs_url_construction_consistency() {
    // Verify docs module uses github module correctly
    let docs_url = docs::build_docs_archive_url("0.12.0").unwrap();
    let expected = "https://github.com/typst/typst/archive/refs/tags/v0.12.0.tar.gz";
    assert_eq!(docs_url.as_str(), expected);
}

#[test]
fn test_docs_max_size_constant() {
    // Verify MAX_DOCS_SIZE is accessible and reasonable
    assert_eq!(docs::MAX_DOCS_SIZE, 50 * 1024 * 1024); // 50 MB
}

#[test]
fn test_github_user_agent_constant() {
    // Verify USER_AGENT is set correctly
    assert_eq!(github::USER_AGENT, "typstlab");
}

#[test]
fn test_github_default_timeout() {
    use std::time::Duration;
    // Verify DEFAULT_TIMEOUT is reasonable
    assert_eq!(github::DEFAULT_TIMEOUT, Duration::from_secs(30));
}

// Note: Actual download tests (full integration) are in existing docs_tests.rs
// These tests focus on library API correctness and safety properties
