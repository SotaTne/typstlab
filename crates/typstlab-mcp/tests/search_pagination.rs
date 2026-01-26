use serde_json::Value;
use typstlab_core::config::consts::search::MAX_MATCHES;
use typstlab_mcp::context::McpContext;
use typstlab_mcp::handlers::docs::{DocsSearchArgs, DocsTool};
use typstlab_mcp::server::TypstlabServer;
use typstlab_testkit::temp_dir_in_workspace;

fn parse_result(result: &rmcp::model::CallToolResult) -> Value {
    let text = result.content[0].as_text().unwrap();
    serde_json::from_str(&text.text).unwrap()
}

#[tokio::test]
async fn test_docs_search_pagination_returns_first_page() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();

    // Create enough files to exceed one page (MAX_MATCHES=50)
    // We create 60 files, each with 1 match.
    for i in 0..60 {
        tokio::fs::write(docs_dir.join(format!("f{:03}.md", i)), "search_target")
            .await
            .unwrap();
    }

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // Page 1
    let res = DocsTool::test_docs_search(
        &server,
        DocsSearchArgs {
            query: "search_target".to_string(),
            page: 1, // Default, but explicit here
        },
    )
    .await
    .unwrap();

    let json = parse_result(&res);
    let matches = json["matches"].as_array().unwrap();

    assert_eq!(
        matches.len(),
        MAX_MATCHES,
        "Should return MAX_MATCHES items on page 1"
    );
    assert_eq!(
        json["truncated"], true,
        "Should be truncated as more items exist"
    );
}

#[tokio::test]
async fn test_docs_search_pagination_returns_second_page() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();

    // Create 60 files
    for i in 0..60 {
        tokio::fs::write(docs_dir.join(format!("f{:03}.md", i)), "search_target")
            .await
            .unwrap();
    }

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);

    // Page 2
    let res = DocsTool::test_docs_search(
        &server,
        DocsSearchArgs {
            query: "search_target".to_string(),
            page: 2,
        },
    )
    .await
    .unwrap();

    let json = parse_result(&res);
    let matches = json["matches"].as_array().unwrap();

    // We expect 60 - 50 = 10 items
    assert_eq!(
        matches.len(),
        10,
        "Should return remaining 10 items on page 2"
    );
    // Should NOT be truncated (no more items)
    // Wait, implementation might set truncated=false if exact match? or check if scan reached end?
    // If total found is 60, and page 2 returns 10 (idx 50-59), and scan completed.
    // Truncation usually means "did we stop effectively before finishing everything OR did we hit limit?"
    // If we return partial page (10 < 50), it means we exhausted results. So truncated=false.
    // If we returned 50 items, we check if there are more? The current logic stops when limit reached.
    // To know "more exist", we usually scan limit+1.
    // We will see how implementation goes. For now expect false or implement limit+1 logic.
    // User requirement: "page returns what was not included... return truncated=true then page+1".
    // If page 2 clears the rest, truncated should be false.
    assert_eq!(json["truncated"], false);
}

#[tokio::test]
async fn test_docs_search_pagination_integrity() {
    let temp = temp_dir_in_workspace();
    let docs_dir = temp.path().join(".typstlab/kb/typst/docs");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();

    // Create 60 files with predictable names
    let mut expected_paths = Vec::new();
    for i in 0..60 {
        let filename = format!("f{:03}.md", i);
        tokio::fs::write(docs_dir.join(&filename), "search_target")
            .await
            .unwrap();
        expected_paths.push(filename);
    }
    expected_paths.sort();

    let ctx = McpContext::new(temp.path().to_path_buf());
    let server = TypstlabServer::new(ctx, false);
    let query = "search_target".to_string();

    // Fetch Page 1
    let res1 = DocsTool::test_docs_search(
        &server,
        DocsSearchArgs {
            query: query.clone(),
            page: 1,
        },
    )
    .await
    .unwrap();
    let json1 = parse_result(&res1);

    // Validate Query Reflection (Page 1)
    // The query field returns the full arguments object properly reflected
    assert_eq!(
        json1["query"]["query"], query,
        "Response query string should match input (Page 1)"
    );
    assert_eq!(
        json1["query"]["page"], 1,
        "Response page should be 1 (Page 1)"
    );

    // Fetch Page 2
    let res2 = DocsTool::test_docs_search(
        &server,
        DocsSearchArgs {
            query: query.clone(),
            page: 2,
        },
    )
    .await
    .unwrap();
    let json2 = parse_result(&res2);

    // Validate Query Reflection (Page 2)
    assert_eq!(
        json2["query"]["query"], query,
        "Response query string should match input (Page 2)"
    );
    assert_eq!(
        json2["query"]["page"], 2,
        "Response page should be 2 (Page 2)"
    );

    // Combine matches
    let mut all_matches = Vec::new();
    all_matches.extend(json1["matches"].as_array().unwrap().iter().cloned());
    all_matches.extend(json2["matches"].as_array().unwrap().iter().cloned());

    assert_eq!(all_matches.len(), 60, "Total matches should be 60");

    // Extract filenames from matches and verify uniqueness/completeness
    let mut collected_filenames: Vec<String> = all_matches
        .iter()
        .map(|m| {
            let path_str = m["path"].as_str().unwrap();
            std::path::Path::new(path_str)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    collected_filenames.sort();

    assert_eq!(
        collected_filenames, expected_paths,
        "Combined results should exactly match all created files without duplicates or gaps"
    );
}
