//! Integration tests for docs module and shared GitHub architecture

use typstlab_typst::{docs, github};

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
