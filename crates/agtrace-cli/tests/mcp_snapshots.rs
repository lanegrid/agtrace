use agtrace_sdk::{Client, types::SessionFilter};
use agtrace_testing::TestWorld;
use agtrace_testing::providers::TestProvider;
use anyhow::Result;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// MCP server interaction helper
struct McpHarness {
    process: std::process::Child,
}

impl McpHarness {
    fn new(data_dir: &str) -> Result<Self> {
        let process = Command::new(assert_cmd::cargo::cargo_bin!("agtrace"))
            .arg("mcp")
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

/// Mask dynamic values recursively, including JSON strings
fn mask_recursive(v: &mut Value) {
    match v {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if key == "id"
                    || key == "session_id"
                    || key == "turn_id"
                    || key == "event_id"
                    || key == "tool_call_id"
                    || key == "provider_call_id"
                {
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
        Value::String(s) => {
            // If string looks like JSON, try to parse and mask it
            if ((s.starts_with('{') && s.ends_with('}'))
                || (s.starts_with('[') && s.ends_with(']')))
                && let Ok(mut nested) = serde_json::from_str::<Value>(s)
            {
                mask_recursive(&mut nested);
                if let Ok(masked_str) = serde_json::to_string(&nested) {
                    *s = masked_str;
                }
            }
        }
        _ => {}
    }
}

/// Extract and parse MCP response text content with redaction
fn extract_mcp_text_content(response: &Value) -> Result<Value> {
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .expect("Should have text content");

    let mut content: Value = serde_json::from_str(text)?;
    mask_recursive(&mut content);

    Ok(content)
}

/// Create a snapshot value containing both request and response
fn snapshot_req_resp(request_args: Value, response: Value) -> Value {
    json!({
        "request": request_args,
        "response": response
    })
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

    let args = json!({ "limit": 5 });
    let response = mcp.request(
        "tools/call",
        json!({
            "name": "list_sessions",
            "arguments": args.clone()
        }),
    )?;

    let content = extract_mcp_text_content(&response)?;
    insta::assert_json_snapshot!("call_list_sessions", snapshot_req_resp(args, content));

    Ok(())
}

// Tests for Random Access APIs (list_turns, get_turns)
#[tokio::test]
async fn test_mcp_list_turns() -> Result<()> {
    let world = setup_world()?;

    // Get real session ID using SDK
    let client = Client::connect(world.data_dir()).await?;
    let sessions = client.sessions().list(SessionFilter::all())?;
    let session_id = &sessions[0].id;

    let mut mcp = McpHarness::new(world.data_dir().to_str().unwrap())?;

    let response = mcp.request(
        "tools/call",
        json!({
            "name": "list_turns",
            "arguments": { "session_id": session_id }
        }),
    )?;

    let content = extract_mcp_text_content(&response)?;
    let args = json!({ "session_id": "[ID]" });
    insta::assert_json_snapshot!("call_list_turns", snapshot_req_resp(args, content));

    Ok(())
}

#[tokio::test]
async fn test_mcp_get_turns() -> Result<()> {
    let world = setup_world()?;

    // Get real session ID using SDK
    let client = Client::connect(world.data_dir()).await?;
    let sessions = client.sessions().list(SessionFilter::all())?;
    let session_id = &sessions[0].id;

    let mut mcp = McpHarness::new(world.data_dir().to_str().unwrap())?;

    let response = mcp.request(
        "tools/call",
        json!({
            "name": "get_turns",
            "arguments": { "session_id": session_id, "turn_indices": [0] }
        }),
    )?;

    let content = extract_mcp_text_content(&response)?;
    let args = json!({ "session_id": "[ID]", "turn_indices": [0] });
    insta::assert_json_snapshot!("call_get_turns", snapshot_req_resp(args, content));

    Ok(())
}

#[test]
fn test_mcp_error_invalid_params() -> Result<()> {
    let world = setup_world()?;
    let mut mcp = McpHarness::new(world.data_dir().to_str().unwrap())?;

    let args = json!({});
    let response = mcp.request(
        "tools/call",
        json!({
            "name": "get_turns",
            "arguments": args.clone()
        }),
    )?;

    insta::assert_json_snapshot!("error_invalid_params", snapshot_req_resp(args, response));

    Ok(())
}

#[test]
fn test_mcp_error_session_not_found() -> Result<()> {
    let world = setup_world()?;
    let mut mcp = McpHarness::new(world.data_dir().to_str().unwrap())?;

    let args = json!({ "session_id": "nonexistent-id-12345678" });
    let response = mcp.request(
        "tools/call",
        json!({
            "name": "list_turns",
            "arguments": args.clone()
        }),
    )?;

    insta::assert_json_snapshot!("error_session_not_found", snapshot_req_resp(args, response));

    Ok(())
}
