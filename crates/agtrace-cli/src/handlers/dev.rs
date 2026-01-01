use agtrace_sdk::Client;
use anyhow::Result;
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

pub async fn handle_test_mcp(client: &Client, verbose: bool) -> Result<()> {
    println!("Starting MCP server test...\n");

    let mut server = spawn_mcp_server()?;
    let mut stdin = server.stdin.take().expect("Failed to get stdin");
    let stdout = server.stdout.take().expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    let mut total_warnings = 0;

    // Test 1: Initialize
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 1: initialize");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let (response, size) = send_request(&mut stdin, &mut reader, "initialize", json!({}))?;
    print_result("initialize", size, 10_000, verbose, &response);
    if size > 10_000 {
        total_warnings += 1;
    }

    // Test 2: List tools
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 2: tools/list");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let (response, size) = send_request(&mut stdin, &mut reader, "tools/list", json!({}))?;
    print_result("tools/list", size, 50_000, verbose, &response);
    if size > 50_000 {
        total_warnings += 1;
    }

    // Get a session ID for further tests
    let session_id = get_first_session_id(client)?;

    if let Some(sid) = &session_id {
        // Test 3: list_sessions (default)
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Test 3: list_sessions (default limit: 50)");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        let (response, size) = send_request(
            &mut stdin,
            &mut reader,
            "tools/call",
            json!({
                "name": "list_sessions",
                "arguments": {}
            }),
        )?;
        print_result("list_sessions (default)", size, 100_000, verbose, &response);
        if size > 100_000 {
            total_warnings += 1;
        }

        // Test 4: list_sessions (limit: 5)
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Test 4: list_sessions (limit: 5)");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        let (response, size) = send_request(
            &mut stdin,
            &mut reader,
            "tools/call",
            json!({
                "name": "list_sessions",
                "arguments": { "limit": 5 }
            }),
        )?;
        print_result("list_sessions (limit: 5)", size, 50_000, verbose, &response);
        if size > 50_000 {
            total_warnings += 1;
        }

        // Test 5: get_session_details
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Test 5: get_session_details");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        let (response, size) = send_request(
            &mut stdin,
            &mut reader,
            "tools/call",
            json!({
                "name": "get_session_details",
                "arguments": { "session_id": sid }
            }),
        )?;
        println!("Session ID: {}", sid);

        // Analyze session structure
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&response)
            && let Some(content) = parsed["result"]["content"][0]["text"].as_str()
                && let Ok(session_data) = serde_json::from_str::<serde_json::Value>(content) {
                    if let Some(turns) = session_data["turns"].as_array() {
                        println!("  - turns: {} items", turns.len());
                    }
                    if let Some(events) = session_data["events"].as_array() {
                        println!("  - events: {} items", events.len());
                    }
                }

        print_result("get_session_details", size, 500_000, verbose, &response);
        if size > 500_000 {
            total_warnings += 1;
        }

        // Test 6: analyze_session
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Test 6: analyze_session");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        let (response, size) = send_request(
            &mut stdin,
            &mut reader,
            "tools/call",
            json!({
                "name": "analyze_session",
                "arguments": {
                    "session_id": sid,
                    "include_failures": true,
                    "include_loops": false
                }
            }),
        )?;
        print_result("analyze_session", size, 200_000, verbose, &response);
        if size > 200_000 {
            total_warnings += 1;
        }

        // Test 7: search_events
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Test 7: search_events (pattern: 'Read', limit: 5)");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        let (response, size) = send_request(
            &mut stdin,
            &mut reader,
            "tools/call",
            json!({
                "name": "search_events",
                "arguments": {
                    "pattern": "Read",
                    "limit": 5
                }
            }),
        )?;
        print_result("search_events", size, 100_000, verbose, &response);
        if size > 100_000 {
            total_warnings += 1;
        }
    } else {
        println!("\n⚠️  No sessions found. Skipping session-specific tests.");
    }

    // Test 8: get_project_info
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 8: get_project_info");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let (response, size) = send_request(
        &mut stdin,
        &mut reader,
        "tools/call",
        json!({
            "name": "get_project_info",
            "arguments": {}
        }),
    )?;
    print_result("get_project_info", size, 50_000, verbose, &response);
    if size > 50_000 {
        total_warnings += 1;
    }

    // Summary
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Summary");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    if total_warnings == 0 {
        println!("✅ All tests passed! No response size warnings.");
    } else {
        println!(
            "⚠️  {} test(s) exceeded recommended size limits.",
            total_warnings
        );
        println!("\nRecommendations:");
        println!("  1. Implement pagination for large result sets");
        println!("  2. Add size limits or truncation for verbose fields");
        println!("  3. Consider streaming responses for large payloads");
    }

    server.kill()?;

    Ok(())
}

fn spawn_mcp_server() -> Result<Child> {
    let exe = std::env::current_exe()?;
    let child = Command::new(exe)
        .arg("serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(child)
}

fn send_request(
    stdin: &mut std::process::ChildStdin,
    reader: &mut BufReader<std::process::ChildStdout>,
    method: &str,
    params: serde_json::Value,
) -> Result<(String, usize)> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });

    writeln!(stdin, "{}", serde_json::to_string(&request)?)?;
    stdin.flush()?;

    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;
    let size = response_line.len();

    Ok((response_line, size))
}

fn print_result(_test_name: &str, size: usize, threshold: usize, verbose: bool, response: &str) {
    let size_kb = size as f64 / 1024.0;

    let status = if size > threshold {
        "⚠️  WARNING"
    } else {
        "✅ OK"
    };

    println!("Result: {} - {:.2} KB ({} bytes)", status, size_kb, size);

    if size > threshold {
        let ratio = size as f64 / threshold as f64;
        println!(
            "  Exceeds threshold by {:.1}x ({} KB limit)",
            ratio,
            threshold / 1024
        );
    }

    if verbose {
        println!("\nResponse preview (first 500 chars):");
        println!("{}", &response.chars().take(500).collect::<String>());
        if response.len() > 500 {
            println!("... (truncated)");
        }
    }
}

fn get_first_session_id(client: &Client) -> Result<Option<String>> {
    use agtrace_sdk::SessionFilter;

    let sessions = client.sessions().list(SessionFilter::all().limit(1))?;

    Ok(sessions.first().map(|s| s.id.clone()))
}
