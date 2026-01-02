use agtrace_testing::providers::TestProvider;
use agtrace_testing::TestWorld;
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// MCP server interaction helper
struct McpHarness {
    process: std::process::Child,
}

impl McpHarness {
    fn new(data_dir: &str) -> Result<Self> {
        let process = Command::new(assert_cmd::cargo::cargo_bin("agtrace"))
            .arg("serve")
            .arg("--data-dir")
            .arg(data_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        Ok(Self { process })
    }

    /// Send request and receive response
    fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

        let stdin = self.process.stdin.as_mut().expect("Failed to open stdin");
        writeln!(stdin, "{}", serde_json::to_string(&request)?)?;

        let stdout = self.process.stdout.as_mut().expect("Failed to open stdout");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        let response: Value = serde_json::from_str(&line)?;
        Ok(response)
    }
}

impl Drop for McpHarness {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}

/// Redact dynamic values in nested MCP JSON content
fn redact_mcp_content(value: insta::internals::Content, _path: insta::internals::ContentPath) -> String {
    let text = match value {
        insta::internals::Content::String(s) => s,
        _ => return "[NON_STRING_CONTENT]".to_string(),
    };

    let mut inner_json: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return "[NON_JSON_CONTENT]".to_string(),
    };

    fn mask_recursive(v: &mut Value) {
        match v {
            Value::Object(map) => {
                for (key, val) in map.iter_mut() {
                    if key == "id" || key == "session_id" || key == "turn_id" {
                        *val = serde_json::json!("[ID]");
                    } else if key.contains("time") || key == "timestamp" {
                        *val = serde_json::json!("[TIMESTAMP]");
                    } else if key.contains("path") || key == "root" || key == "project_root" {
                        *val = serde_json::json!("[PATH]");
                    } else if key == "project_hash" {
                        *val = serde_json::json!("[PROJECT_HASH]");
                    } else {
                        mask_recursive(val);
                    }
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    mask_recursive(item);
                }
            }
            _ => {}
        }
    }

    mask_recursive(&mut inner_json);
    serde_json::to_string_pretty(&inner_json).unwrap()
}

/// Setup test world with sample session data
fn setup_world() -> Result<TestWorld> {
    let mut world = TestWorld::new().with_project("my-project");
    world.enable_provider(TestProvider::Claude)?;
    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude_session.jsonl")?;
    world.run(&["init"])?;
    Ok(world)
}

// -----------------------------------------------------------------------------
// Test Cases
// -----------------------------------------------------------------------------

#[test]
fn test_mcp_initialize() -> Result<()> {
    let world = setup_world()?;
    let mut mcp = McpHarness::new(world.data_dir().to_str().unwrap())?;

    let response = mcp.request("initialize", json!({}))?;

    insta::assert_json_snapshot!("initialize", response, {
        ".result.serverInfo.version" => "[VERSION]"
    });

    Ok(())
}

#[test]
fn test_mcp_tools_list() -> Result<()> {
    let world = setup_world()?;
    let mut mcp = McpHarness::new(world.data_dir().to_str().unwrap())?;

    let _ = mcp.request("initialize", json!({}))?;
    let response = mcp.request("tools/list", json!({}))?;

    insta::assert_json_snapshot!("tools_list", response);

    Ok(())
}

#[test]
fn test_mcp_list_sessions() -> Result<()> {
    let world = setup_world()?;
    let mut mcp = McpHarness::new(world.data_dir().to_str().unwrap())?;

    let response = mcp.request(
        "tools/call",
        json!({
            "name": "list_sessions",
            "arguments": { "limit": 5 }
        }),
    )?;

    insta::assert_json_snapshot!("call_list_sessions", response, {
        ".result.content[0].text" => insta::dynamic_redaction(redact_mcp_content)
    });

    Ok(())
}

#[test]
fn test_mcp_get_session_summary() -> Result<()> {
    let world = setup_world()?;

    let list_output = world.run(&["session", "list", "--format", "json"])?;
    let list_json = list_output.json()?;
    let session_id = list_json["content"]["sessions"][0]["id"]
        .as_str()
        .expect("Should have session ID");

    let mut mcp = McpHarness::new(world.data_dir().to_str().unwrap())?;

    let response = mcp.request(
        "tools/call",
        json!({
            "name": "get_session_summary",
            "arguments": { "session_id": session_id }
        }),
    )?;

    insta::assert_json_snapshot!("call_get_session_summary", response, {
        ".result.content[0].text" => insta::dynamic_redaction(redact_mcp_content)
    });

    Ok(())
}

#[test]
fn test_mcp_get_session_turns() -> Result<()> {
    let world = setup_world()?;

    let list_output = world.run(&["session", "list", "--format", "json"])?;
    let list_json = list_output.json()?;
    let session_id = list_json["content"]["sessions"][0]["id"]
        .as_str()
        .expect("Should have session ID");

    let mut mcp = McpHarness::new(world.data_dir().to_str().unwrap())?;

    let response = mcp.request(
        "tools/call",
        json!({
            "name": "get_session_turns",
            "arguments": { "session_id": session_id, "limit": 3 }
        }),
    )?;

    insta::assert_json_snapshot!("call_get_session_turns", response, {
        ".result.content[0].text" => insta::dynamic_redaction(redact_mcp_content)
    });

    Ok(())
}

#[test]
fn test_mcp_error_invalid_params() -> Result<()> {
    let world = setup_world()?;
    let mut mcp = McpHarness::new(world.data_dir().to_str().unwrap())?;

    let response = mcp.request(
        "tools/call",
        json!({
            "name": "get_session_summary",
            "arguments": {}
        }),
    )?;

    insta::assert_json_snapshot!("error_invalid_params", response);

    Ok(())
}

#[test]
fn test_mcp_error_session_not_found() -> Result<()> {
    let world = setup_world()?;
    let mut mcp = McpHarness::new(world.data_dir().to_str().unwrap())?;

    let response = mcp.request(
        "tools/call",
        json!({
            "name": "get_session_summary",
            "arguments": { "session_id": "nonexistent-id-12345678" }
        }),
    )?;

    insta::assert_json_snapshot!("error_session_not_found", response);

    Ok(())
}
