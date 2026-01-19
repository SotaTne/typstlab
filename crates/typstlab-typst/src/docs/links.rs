//! Link rewriting utilities for documentation generation.

use std::borrow::Cow;

/// Rewrites a `/DOCS-BASE/` URL to a relative `.md` link.
///
/// # Conversion Rules
///
/// - Root: `/DOCS-BASE/` → `../index.md`
/// - Directories: `/DOCS-BASE/tutorial/` → `../tutorial.md`
/// - Files: `/DOCS-BASE/tutorial/writing` → `../tutorial/writing.md`
/// - Fragments: `/DOCS-BASE/tutorial/#section` → `../tutorial.md#section`
/// - Query strings: `/DOCS-BASE/api?v=1` → `../api.md?v=1`
/// - External URLs: `https://...` → unchanged
/// - Other schemes: `mailto:`, `tel:`, `#...` → unchanged
///
/// # Examples
///
/// ```
/// use typstlab_typst::docs::rewrite_docs_link;
/// use std::borrow::Cow;
///
/// assert_eq!(rewrite_docs_link("/DOCS-BASE/", 1), Cow::Borrowed("../index.md"));
/// assert_eq!(rewrite_docs_link("/DOCS-BASE/tutorial/", 1), Cow::Borrowed("../tutorial.md"));
/// assert_eq!(rewrite_docs_link("https://example.com", 1), Cow::Borrowed("https://example.com"));
/// ```
pub fn rewrite_docs_link(url: &str, depth: usize) -> Cow<'_, str> {
    // 1. 外部URL、スキーム付きURL、フラグメントのみは変更なし
    if url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("mailto:")
        || url.starts_with("tel:")
        || url.starts_with('#')
    {
        return Cow::Borrowed(url);
    }

    // 2. /DOCS-BASE/ で始まらない場合は変更なし
    if !url.starts_with("/DOCS-BASE/") {
        return Cow::Borrowed(url);
    }

    // 3. パス、クエリ、フラグメントを分離
    let after_base = &url["/DOCS-BASE/".len()..];

    let (path_part, fragment_part) = match after_base.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment)),
        None => (after_base, None),
    };

    let (path_part, query_part) = match path_part.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (path_part, None),
    };

    // 4. セキュリティ: パストラバーサル防止
    if path_part.contains("..") {
        return Cow::Borrowed(url);
    }

    // 5. パスを変換
    let md_path = if path_part.is_empty() || path_part == "/" {
        "index"
    } else {
        path_part.trim_end_matches('/')
    };

    // 6. 結果を構築
    let prefix = "../".repeat(depth);
    let mut result = format!("{}{}.md", prefix, md_path);

    if let Some(query) = query_part {
        result.push('?');
        result.push_str(query);
    }

    if let Some(fragment) = fragment_part {
        result.push('#');
        result.push_str(fragment);
    }

    Cow::Owned(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_docs_link_root() {
        assert_eq!(rewrite_docs_link("/DOCS-BASE/", 1), "../index.md");
    }

    #[test]
    fn test_rewrite_docs_link_directory_with_trailing_slash() {
        assert_eq!(
            rewrite_docs_link("/DOCS-BASE/tutorial/", 1),
            "../tutorial.md"
        );
    }

    #[test]
    fn test_rewrite_docs_link_nested_directory() {
        assert_eq!(
            rewrite_docs_link("/DOCS-BASE/tutorial/writing/", 1),
            "../tutorial/writing.md"
        );
    }

    #[test]
    fn test_rewrite_docs_link_file_without_trailing_slash() {
        assert_eq!(
            rewrite_docs_link("/DOCS-BASE/tutorial/writing", 1),
            "../tutorial/writing.md"
        );
    }

    #[test]
    fn test_rewrite_docs_link_with_fragment() {
        assert_eq!(
            rewrite_docs_link("/DOCS-BASE/tutorial/#section", 1),
            "../tutorial.md#section"
        );
    }

    #[test]
    fn test_rewrite_docs_link_with_query() {
        assert_eq!(
            rewrite_docs_link("/DOCS-BASE/api?version=1", 1),
            "../api.md?version=1"
        );
    }

    #[test]
    fn test_rewrite_docs_link_with_query_and_fragment() {
        assert_eq!(
            rewrite_docs_link("/DOCS-BASE/api?v=1#intro", 1),
            "../api.md?v=1#intro"
        );
    }

    #[test]
    fn test_rewrite_docs_link_https_unchanged() {
        assert_eq!(
            rewrite_docs_link("https://example.com", 1),
            "https://example.com"
        );
    }

    #[test]
    fn test_rewrite_docs_link_mailto_unchanged() {
        assert_eq!(
            rewrite_docs_link("mailto:test@example.com", 1),
            "mailto:test@example.com"
        );
    }

    #[test]
    fn test_rewrite_docs_link_fragment_only_unchanged() {
        assert_eq!(rewrite_docs_link("#section", 1), "#section");
    }

    #[test]
    fn test_rewrite_docs_link_non_docs_base_unchanged() {
        assert_eq!(rewrite_docs_link("/other/path", 1), "/other/path");
    }

    #[test]
    fn test_rewrite_docs_link_path_traversal_blocked() {
        // セキュリティ: パストラバーサルは変換しない
        assert_eq!(
            rewrite_docs_link("/DOCS-BASE/../etc/passwd", 1),
            "/DOCS-BASE/../etc/passwd"
        );
    }

    #[test]
    fn test_rewrite_docs_link_url_encoded_preserved() {
        assert_eq!(
            rewrite_docs_link("/DOCS-BASE/tutorial%20guide/", 1),
            "../tutorial%20guide.md"
        );
    }

    #[test]
    fn test_rewrite_docs_link_depth_2() {
        // Verify depth 2 works (../../)
        assert_eq!(
            rewrite_docs_link("/DOCS-BASE/tutorial/", 2),
            "../../tutorial.md"
        );
    }
}
