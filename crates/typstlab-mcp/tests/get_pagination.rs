//! Get Pagination Tests
//!
//! Verifies that get results are correctly paginated by lines.

use serde_json::Value;
use typstlab_core::config::consts::get::MAX_GET_LINES;
use typstlab_mcp::context::McpContext;
use typstlab_mcp::handlers::docs::{DocsGetArgs, DocsTool};
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;

fn parse_result(result: &rmcp::model::CallToolResult) -> Value {
    let text = result.content[0].as_text().unwrap();
    // docs_get returns raw text content, no wrapping JSON unless error?
    // Wait, docs_get returns text.
    // So the 'text' field IS the file content.
    // It's not JSON unless we structured it.
    // The test helper `parse_result` in other tests assumes JSON.
    // But `docs_get` returns plain text of the file.
    // So we don't parse as JSON.
    Value::String(text.text.clone())
}

#[tokio::test]
async fn test_docs_get_pagination_page1() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();

    // Create a large file (exceeding MAX_GET_LINES=100)
    let mut content = String::new();
    for i in 1..=150 {
        content.push_str(&format!("Line {}\n", i));
    }
    tokio::fs::write(docs_dir.join("large.md"), &content)
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // Page 1: Should return lines 1-100
    let res = DocsTool::docs_get(
        &server,
        DocsGetArgs {
            path: "large.md".to_string(),
            page: 1,
        },
    )
    .await
    .unwrap();

    let text = res.content[0].as_text().unwrap().text.clone();
    let lines: Vec<&str> = text.lines().collect();

    assert_eq!(
        lines.len(),
        MAX_GET_LINES,
        "Page 1 should have MAX_GET_LINES"
    );
    assert_eq!(lines[0], "Line 1");
    // Line 100
    assert_eq!(lines[99], "Line 100");
}

#[tokio::test]
async fn test_docs_get_pagination_page2() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();

    let mut content = String::new();
    for i in 1..=150 {
        content.push_str(&format!("Line {}\n", i));
    }
    tokio::fs::write(docs_dir.join("large.md"), &content)
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // Page 2: Should return lines 101-150
    let res = DocsTool::docs_get(
        &server,
        DocsGetArgs {
            path: "large.md".to_string(),
            page: 2,
        },
    )
    .await
    .unwrap();

    let text = res.content[0].as_text().unwrap().text.clone();
    let lines: Vec<&str> = text.lines().collect();

    assert_eq!(lines.len(), 50, "Page 2 should have remaining 50 lines");
    assert_eq!(lines[0], "Line 101");
    assert_eq!(lines[49], "Line 150");
}

#[tokio::test]
async fn test_docs_get_pagination_page_out_of_bounds() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();

    tokio::fs::write(docs_dir.join("small.md"), "Line 1\n")
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // Page 2: Should return empty string (or error?)
    // Standard pagination returns empty if out of bounds.
    let res = DocsTool::docs_get(
        &server,
        DocsGetArgs {
            path: "small.md".to_string(),
            page: 2,
        },
    )
    .await
    .unwrap();

    let text = res.content[0].as_text().unwrap().text.clone();
    assert!(
        text.is_empty(),
        "Out of bounds page should return empty text"
    );
}

#[tokio::test]
async fn test_rules_get_pagination() {
    let temp = temp_dir_in_workspace();
    let rules_dir = temp.path().join("rules");
    tokio::fs::create_dir_all(&rules_dir).await.unwrap();

    let mut content = String::new();
    for i in 1..=150 {
        content.push_str(&format!("Line {}\n", i));
    }
    tokio::fs::write(rules_dir.join("large.md"), &content)
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    use typstlab_mcp::handlers::rules::{RulesGetArgs, RulesTool};

    // Page 2: Should return lines 101-150
    let res = RulesTool::rules_get(
        &server,
        RulesGetArgs {
            path: "rules/large.md".to_string(),
            page: 2,
        },
    )
    .await
    .unwrap();

    let text = res.content[0].as_text().unwrap().text.clone();
    let lines: Vec<&str> = text.lines().collect();

    assert_eq!(lines.len(), 50, "Rules page 2 should have 50 lines");
    assert_eq!(lines[0], "Line 101");
}

#[tokio::test]
async fn test_docs_get_pagination_reassembly() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();

    // Create content with 150 lines
    let mut original_content = String::new();
    for i in 1..=150 {
        original_content.push_str(&format!("Line {}\n", i));
    }
    tokio::fs::write(docs_dir.join("reassembly.md"), &original_content)
        .await
        .unwrap();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // Page 1
    let res1 = DocsTool::docs_get(
        &server,
        DocsGetArgs {
            path: "reassembly.md".to_string(),
            page: 1,
        },
    )
    .await
    .unwrap();
    let text1 = res1.content[0].as_text().unwrap().text.clone();

    // Page 2
    let res2 = DocsTool::docs_get(
        &server,
        DocsGetArgs {
            path: "reassembly.md".to_string(),
            page: 2,
        },
    )
    .await
    .unwrap();
    let text2 = res2.content[0].as_text().unwrap().text.clone();

    // Reassemble
    // Note: get returns lines joined by \n.
    // If the original file ended with \n, the joined string might arguably miss the final \n
    // depending on implementation (lines() iterator consumes delimiters).
    // Our implementation: `lines()...join("\n")`.
    // Effectively `lines()` strips `\n`, `join` puts them back between items.
    // It does NOT add a trailing newline if the original had one?
    // `lines()` behavior: "The final line ending is optional."
    // `join("\n")` creates "Line 1\n...\nLine 100". No trailing newline.
    // `text1` + `\n` + `text2` logic might be needed?
    // Wait, typical usage: Page 1 gives lines 1..100. Page 2 gives 101..150.
    // If we just concat them blindly?
    // text1: "Line 1\n...Line 100" (no trailing newline)
    // text2: "Line 101\n...Line 150" (no trailing newline)
    // Reassembled: text1 + "\n" + text2 + (maybe "\n" if we want exact file match?)

    // Let's verify exactly what we get.
    let reassembled = format!("{}\n{}", text1, text2);

    // The original content has a trailing newline after every line "Line X\n".
    // So "Line 150\n".
    // Our reassembled will change "Line 150\n" to "Line 150" (no newline) because `lines()` stripping.
    // That is acceptable for text retrieval mostly, but let's verifying strictly what matches.

    let original_trimmed = original_content.trim(); // Remove final newline for comparison

    assert_eq!(
        reassembled, original_trimmed,
        "Reassembled paginated content should match original content (ignoring final newline)"
    );
}
