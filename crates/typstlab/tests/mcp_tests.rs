use assert_cmd::Command;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

/// Helper: Create a temporary typstlab project
fn create_test_project() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
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

/// Helper: Create a dummy doc file for search tests
fn setup_docs(project: &TempDir) {
    let docs_dir = project.path().join(".typstlab/kb/typst/docs");
    fs::create_dir_all(&docs_dir).expect("Failed to create docs dir");
    fs::write(
        docs_dir.join("intro.md"),
        "Welcome to Typst documentation.\nThis is a search target.",
    )
    .expect("Failed to write dummy doc");
}

#[test]
fn test_mcp_test_tools_list() {
    let project = create_test_project();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_typstlab"));

    let input = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 1
    });

    let assert = cmd
        .arg("mcp")
        .arg("stdio")
        .current_dir(project.path())
        .write_stdin(input.to_string() + "\n")
        .assert();

    let assert = assert.success();
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Filter for the JSON response line (ignore "MCP Server running..." logs)
    let json_line = stdout
        .lines()
        .find(|l| l.starts_with('{'))
        .expect("Should contain JSON response");

    let response: serde_json::Value =
        serde_json::from_str(json_line).expect("Should be valid JSON");

    assert_eq!(response["id"], 1);
    assert!(response["result"]["tools"].as_array().unwrap().len() >= 2);

    let tools = response["result"]["tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t["name"] == "docs_search"));
    assert!(tools.iter().any(|t| t["name"] == "rules_list"));
}

#[test]
fn test_mcp_resources_list() {
    let project = create_test_project();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_typstlab"));

    let input = json!({
        "jsonrpc": "2.0",
        "method": "resources/list",
        "id": 2
    });

    let assert = cmd
        .arg("mcp")
        .arg("stdio")
        .current_dir(project.path())
        .write_stdin(input.to_string() + "\n")
        .assert();

    let assert = assert.success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    let json_line = stdout
        .lines()
        .find(|l| l.starts_with('{'))
        .expect("Should contain JSON response");

    let response: serde_json::Value =
        serde_json::from_str(json_line).expect("Should be valid JSON");

    assert_eq!(response["id"], 2);
    let resources = response["result"]["resources"].as_array().unwrap();
    assert!(resources.iter().any(|r| r["uri"] == "typstlab://rules"));
}

#[test]
fn test_mcp_tools_call_docs_search() {
    let project = create_test_project();
    setup_docs(&project);

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_typstlab"));

    let input = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "docs_search",
            "arguments": {
                "query": "search target"
            }
        },
        "id": 3
    });

    let assert = cmd
        .arg("mcp")
        .arg("stdio")
        .current_dir(project.path())
        .write_stdin(input.to_string() + "\n")
        .assert();

    let assert = assert.success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    let json_line = stdout
        .lines()
        .find(|l| l.starts_with('{'))
        .expect("Should contain JSON response");

    let response: serde_json::Value =
        serde_json::from_str(json_line).expect("Should be valid JSON");

    assert_eq!(response["id"], 3);
    let matches = response["result"]["matches"].as_array().unwrap();
    assert!(!matches.is_empty(), "Should return matches");
    assert_eq!(matches[0]["path"], "intro.md");
    assert!(
        matches[0]["excerpt"]
            .as_str()
            .unwrap()
            .contains("search target")
    );
}

#[test]
fn test_mcp_resources_read_rules() {
    let project = create_test_project();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_typstlab"));

    let input = json!({
        "jsonrpc": "2.0",
        "method": "resources/read",
        "params": {
            "uri": "typstlab://rules"
        },
        "id": 4
    });

    let assert = cmd
        .arg("mcp")
        .arg("stdio")
        .current_dir(project.path())
        .write_stdin(input.to_string() + "\n")
        .assert();

    let assert = assert.success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    let json_line = stdout
        .lines()
        .find(|l| l.starts_with('{'))
        .expect("Should contain JSON response");

    let response: serde_json::Value =
        serde_json::from_str(json_line).expect("Should be valid JSON");

    assert_eq!(response["id"], 4);
    let contents = response["result"]["contents"].as_array().unwrap();
    assert!(!contents.is_empty());
    assert_eq!(contents[0]["uri"], "typstlab://rules");

    let text = contents[0]["text"].as_str().unwrap();
    assert!(text.contains("test-project"));
}

#[test]
fn test_mcp_tools_call_docs_browse() {
    let project = create_test_project();
    setup_docs(&project);

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_typstlab"));

    let input = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "docs_browse",
            "arguments": {
                "path": "."
            }
        },
        "id": 5
    });

    let assert = cmd
        .arg("mcp")
        .arg("stdio")
        .current_dir(project.path())
        .write_stdin(input.to_string() + "\n")
        .assert();

    let assert = assert.success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    let json_line = stdout
        .lines()
        .find(|l| l.starts_with('{'))
        .expect("Should contain JSON response");

    let response: serde_json::Value =
        serde_json::from_str(json_line).expect("Should be valid JSON");

    assert_eq!(response["id"], 5);
    let items = response["result"]["items"].as_array().unwrap();
    assert!(!items.is_empty(), "Should return items");

    // Check if intro.md is in the list
    assert!(
        items
            .iter()
            .any(|i| i["name"] == "intro.md" && i["type"] == "file")
    );
}
