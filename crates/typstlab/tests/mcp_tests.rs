use anyhow::Result;
use rmcp::model::{CallToolRequestParams, JsonObject, ReadResourceRequestParams};
use rmcp::transport::TokioChildProcess;
use rmcp::{RoleClient, ServiceExt};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use tokio::process::Command;
use typstlab_testkit::temp_dir_in_workspace;

fn create_test_project() -> TempDir {
    let temp_dir = temp_dir_in_workspace();
    let config_path = temp_dir.path().join("typstlab.toml");

    let minimal_config = r#"
[project]
name = "test-project"
init_date = "2026-01-20"

[typst]
version = "0.12.0"
"#;

    fs::write(&config_path, minimal_config).expect("Failed to write config");
    temp_dir
}

fn setup_docs(project: &TempDir) {
    let docs_dir = project.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).expect("Failed to create docs dir");
    fs::write(
        docs_dir.join("intro.md"),
        "Welcome to Typst documentation. Search target here.",
    )
    .expect("Failed to write docs file");
    fs::create_dir_all(docs_dir.join("reference")).expect("Failed to create docs reference dir");
    fs::write(docs_dir.join("reference/syntax.md"), "Syntax guide entry.")
        .expect("Failed to write docs syntax file");
}

async fn connect_mcp(
    project_root: &Path,
    extra_args: &[&str],
) -> Result<rmcp::service::RunningService<RoleClient, ()>> {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_typstlab"));
    cmd.arg("mcp").arg("stdio");
    cmd.args(extra_args);
    cmd.current_dir(project_root);

    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;
    Ok(service)
}

fn json_args(value: Value) -> JsonObject {
    rmcp::model::object(value)
}

fn structured_payload(result: &rmcp::model::CallToolResult) -> Value {
    if let Some(payload) = result.structured_content.clone() {
        return payload;
    }

    if let Some(first) = result.content.first()
        && let Some(text) = first.as_text().map(|t| &t.text)
        && let Ok(parsed) = serde_json::from_str(text)
    {
        return parsed;
    }

    Value::Null
}

#[tokio::test]
async fn test_mcp_tools_list_includes_expected_tools() -> Result<()> {
    let project = create_test_project();
    let service = connect_mcp(project.path(), &[]).await?;

    let tools = service.list_all_tools().await?;
    let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();

    for expected in [
        "cmd_build",
        "docs_browse",
        "docs_search",
    ] {
        assert!(names.contains(&expected), "Missing tool: {expected}");
    }

    service.cancel().await.ok();
    Ok(())
}

#[tokio::test]
async fn test_mcp_tools_list_offline_filters_network_tools() -> Result<()> {
    let project = create_test_project();
    let service = connect_mcp(project.path(), &["--offline"]).await?;

    let tools = service.list_all_tools().await?;
    let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();
    assert!(!names.contains(&"cmd_build"));

    service.cancel().await.ok();
    Ok(())
}

#[tokio::test]
async fn test_mcp_resources_list_includes_docs() -> Result<()> {
    let project = create_test_project();
    let service = connect_mcp(project.path(), &[]).await?;

    let resources = service.list_all_resources().await?;
    assert!(resources.iter().any(|res| res.uri == "typstlab://docs"));

    service.cancel().await.ok();
    Ok(())
}

#[tokio::test]
async fn test_mcp_docs_browse_and_search() -> Result<()> {
    let project = create_test_project();
    setup_docs(&project);
    let service = connect_mcp(project.path(), &[]).await?;

    let browse_result = service
        .call_tool(CallToolRequestParams {
            meta: None,
            name: "docs_browse".into(),
            arguments: Some(json_args(json!({ "path": "" }))),
            task: None,
        })
        .await?;

    let browse_payload = structured_payload(&browse_result);
    let items = browse_payload["items"]
        .as_array()
        .expect("items should be array");
    assert!(items.iter().any(|item| item["name"] == "intro.md"));

    let search_result = service
        .call_tool(CallToolRequestParams {
            meta: None,
            name: "docs_search".into(),
            arguments: Some(json_args(json!({ "query": "search target" }))),
            task: None,
        })
        .await?;

    let search_payload = structured_payload(&search_result);
    let matches = search_payload["matches"]
        .as_array()
        .expect("matches should be array");
    assert!(matches.iter().any(|item| item["path"] == "intro.md"));

    service.cancel().await.ok();
    Ok(())
}

#[tokio::test]
async fn test_mcp_read_resource_docs() -> Result<()> {
    let project = create_test_project();
    setup_docs(&project);
    let service = connect_mcp(project.path(), &[]).await?;

    let docs_result = service
        .read_resource(ReadResourceRequestParams {
            uri: "typstlab://docs/docs/intro.md".into(),
            meta: None,
        })
        .await?;
    let docs_text = match &docs_result.contents[0] {
        rmcp::model::ResourceContents::TextResourceContents { text, .. } => text,
        _ => panic!("Expected text resource contents"),
    };
    assert!(docs_text.contains("Typst documentation"));

    service.cancel().await.ok();
    Ok(())
}

#[tokio::test]
async fn test_mcp_read_resource_rejects_non_markdown() -> Result<()> {
    let project = create_test_project();
    let docs_dir = project.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).expect("Failed to create docs dir");
    fs::write(docs_dir.join("intro.txt"), "plain text").expect("Failed to write docs file");

    let service = connect_mcp(project.path(), &[]).await?;
    let result = service
        .read_resource(ReadResourceRequestParams {
            uri: "typstlab://docs/docs/intro.txt".into(),
            meta: None,
        })
        .await;
    assert!(result.is_err());

    service.cancel().await.ok();
    Ok(())
}

#[tokio::test]
async fn test_mcp_read_resource_rejects_large_file() -> Result<()> {
    let project = create_test_project();
    let docs_dir = project.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).expect("Failed to create docs dir");
    let large_content = vec![b'a'; 1024 * 1024 + 1];
    fs::write(docs_dir.join("large.md"), large_content).expect("Failed to write docs file");

    let service = connect_mcp(project.path(), &[]).await?;
    let result = service
        .read_resource(ReadResourceRequestParams {
            uri: "typstlab://docs/docs/large.md".into(),
            meta: None,
        })
        .await;
    assert!(result.is_err());

    service.cancel().await.ok();
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn test_mcp_read_resource_rejects_symlink_outside_root() -> Result<()> {
    use std::os::unix::fs::symlink;

    let project = create_test_project();
    let docs_dir = project.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).expect("Failed to create docs dir");

    let outside = temp_dir_in_workspace();
    let outside_file = outside.path().join("secret.md");
    fs::write(&outside_file, "secret payload").expect("Failed to write file");
    symlink(&outside_file, docs_dir.join("link.md")).expect("Failed to symlink");

    let service = connect_mcp(project.path(), &[]).await?;
    let result = service
        .read_resource(ReadResourceRequestParams {
            uri: "typstlab://docs/docs/link.md".into(),
            meta: None,
        })
        .await;
    assert!(result.is_err());

    service.cancel().await.ok();
    Ok(())
}

#[tokio::test]
async fn test_mcp_cmd_build_reject_missing_paper() -> Result<()> {
    let project = create_test_project();
    let service = connect_mcp(project.path(), &[]).await?;

    let build_result = service
        .call_tool(CallToolRequestParams {
            meta: None,
            name: "cmd_build".into(),
            arguments: Some(json_args(json!({ "paper_id": "missing" }))),
            task: None,
        })
        .await;
    assert!(build_result.is_err());

    service.cancel().await.ok();
    Ok(())
}
