//! Unit tests for docs module

use typstlab_typst::docs::build_docs_archive_url;

#[test]
fn test_build_docs_archive_url() {
    let url = build_docs_archive_url("0.12.0").unwrap();
    assert_eq!(
        url.as_str(),
        "https://github.com/typst/typst/archive/refs/tags/v0.12.0.tar.gz"
    );
}

#[test]
fn test_build_docs_archive_url_different_version() {
    let url = build_docs_archive_url("0.14.1").unwrap();
    assert_eq!(
        url.as_str(),
        "https://github.com/typst/typst/archive/refs/tags/v0.14.1.tar.gz"
    );
}

#[test]
fn test_url_injection_safety() {
    // Malicious version should be safely encoded
    let url = build_docs_archive_url("../../../etc/passwd").unwrap();
    // The ../ should be URL-encoded, preventing path traversal
    assert!(url.as_str().contains("%2E%2E%2F") || url.as_str().contains("..%2F"));
    assert!(url.as_str().starts_with("https://github.com/typst/typst/"));
}

#[test]
fn test_url_special_characters_encoded() {
    // Special characters should be URL-encoded
    let url = build_docs_archive_url("version with spaces").unwrap();
    assert!(url.as_str().contains("%20"));
    assert!(url.as_str().starts_with("https://github.com/typst/typst/"));
}

#[test]
fn test_build_docs_archive_url_empty_version() {
    // Empty version should still construct valid URL
    let url = build_docs_archive_url("").unwrap();
    assert_eq!(
        url.as_str(),
        "https://github.com/typst/typst/archive/refs/tags/v.tar.gz"
    );
}
