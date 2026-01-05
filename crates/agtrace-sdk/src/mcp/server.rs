//! MCP JSON-RPC server.

use schemars::schema_for;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};

use crate::Client;
use crate::query::{
    AnalyzeSessionArgs, GetTurnsArgs, ListSessionsArgs, ListTurnsArgs, SearchEventsArgs,
};

use super::tools::{
    handle_analyze_session, handle_get_project_info, handle_get_turns, handle_list_sessions,
    handle_list_turns, handle_search_events,
};

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

pub struct AgTraceServer {
    client: Client,
}

impl AgTraceServer {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Convert serde deserialization error to MCP-compliant JSON-RPC error
    fn parse_validation_error(tool_name: &str, error: serde_json::Error) -> JsonRpcError {
        let error_msg = error.to_string();

        // Check if it's a "missing field" error
        if error_msg.contains("missing field") {
            if let Some(field_start) = error_msg.find('`') {
                if let Some(field_end) = error_msg[field_start + 1..].find('`') {
                    let field_name = &error_msg[field_start + 1..field_start + 1 + field_end];
                    return JsonRpcError {
                        code: -32602,
                        message: format!(
                            "Invalid params: missing required field \"{}\"",
                            field_name
                        ),
                        data: Some(json!({
                            "missing": [field_name],
                            "tool": tool_name,
                        })),
                    };
                }
            }
        }

        // Fallback for other validation errors
        JsonRpcError {
            code: -32602,
            message: format!("Invalid params: {}", error),
            data: Some(json!({
                "tool": tool_name,
                "detail": error_msg,
            })),
        }
    }

    async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        // MCP requires all requests to have an id, use a default if missing
        let id = request
            .id
            .clone()
            .unwrap_or_else(|| Value::Number(serde_json::Number::from(0)));

        match request.method.as_str() {
            "initialize" => self.handle_initialize(id, request.params).await,
            "tools/list" => self.handle_list_tools(id).await,
            "tools/call" => self.handle_call_tool(id, request.params).await,
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
            },
        }
    }

    async fn handle_initialize(&self, id: Value, _params: Option<Value>) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "agtrace",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "instructions": "AgTrace MCP Server - AI agent execution observability. Use these tools to query historical sessions, analyze failures, search event payloads, and debug agent behavior."
            })),
            error: None,
        }
    }

    async fn handle_list_tools(&self, id: Value) -> JsonRpcResponse {
        // Generate JSON Schemas from Rust types - single source of truth!
        let list_sessions_schema = schema_for!(ListSessionsArgs);
        let analyze_session_schema = schema_for!(AnalyzeSessionArgs);
        let search_events_schema = schema_for!(SearchEventsArgs);
        let list_turns_schema = schema_for!(ListTurnsArgs);
        let get_turns_schema = schema_for!(GetTurnsArgs);

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "tools": [
                    {
                        "name": "list_sessions",
                        "description": "List recent AI agent sessions with cursor-based pagination. WORKFLOW: Call this first to discover available sessions, then use session IDs with other tools. Safe to call multiple times with different filters.",
                        "inputSchema": serde_json::to_value(&list_sessions_schema).unwrap(),
                    },
                    {
                        "name": "get_project_info",
                        "description": "List all projects that have been indexed by agtrace with their metadata. WORKFLOW: Use this to discover available projects and their hashes. Safe to call anytime.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "analyze_session",
                        "description": "Run diagnostic analysis on a session to identify failures, loops, and issues. WORKFLOW: First call list_sessions to obtain session IDs, then use those IDs with this tool. Safe to call in parallel for multiple known session IDs.",
                        "inputSchema": serde_json::to_value(&analyze_session_schema).unwrap(),
                    },
                    {
                        "name": "search_events",
                        "description": "Search for events and return navigation coordinates (session_id, event_index, turn_index, step_index). Use this to find specific events, then use turn_index with list_turns or get_turns for detailed analysis.",
                        "inputSchema": serde_json::to_value(&search_events_schema).unwrap(),
                    },
                    {
                        "name": "list_turns",
                        "description": "List turns with metadata only (no payload content). Returns turn statistics including step_count, duration_ms, total_tokens, and tools_used. Use this to get an overview before drilling down with get_turns.",
                        "inputSchema": serde_json::to_value(&list_turns_schema).unwrap(),
                    },
                    {
                        "name": "get_turns",
                        "description": "Get details for specific turns. Defaults are tuned for safety based on data distribution (max 30 steps/turn, 3000 chars/field). WORKFLOW: Fetch 1-2 turns at a time to avoid token limits. If data is marked '[TRUNCATED]' and critical, retry with higher limits.",
                        "inputSchema": serde_json::to_value(&get_turns_schema).unwrap(),
                    }
                ]
            })),
            error: None,
        }
    }

    async fn handle_call_tool(&self, id: Value, params: Option<Value>) -> JsonRpcResponse {
        let params = match params {
            Some(p) => p,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing params".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let tool_name = match params.get("name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing tool name".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        let result = match tool_name {
            "list_sessions" => {
                let args: ListSessionsArgs = match serde_json::from_value(arguments) {
                    Ok(args) => args,
                    Err(e) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(Self::parse_validation_error("list_sessions", e)),
                        };
                    }
                };
                handle_list_sessions(&self.client, args).await
            }
            "get_project_info" => handle_get_project_info(&self.client).await,
            "analyze_session" => {
                let args: AnalyzeSessionArgs = match serde_json::from_value(arguments) {
                    Ok(args) => args,
                    Err(e) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(Self::parse_validation_error("analyze_session", e)),
                        };
                    }
                };
                handle_analyze_session(&self.client, args).await
            }
            "search_events" => {
                let args: SearchEventsArgs = match serde_json::from_value(arguments) {
                    Ok(args) => args,
                    Err(e) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(Self::parse_validation_error("search_events", e)),
                        };
                    }
                };
                handle_search_events(&self.client, args).await
            }
            "list_turns" => {
                let args: ListTurnsArgs = match serde_json::from_value(arguments) {
                    Ok(args) => args,
                    Err(e) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(Self::parse_validation_error("list_turns", e)),
                        };
                    }
                };
                handle_list_turns(&self.client, args).await
            }
            "get_turns" => {
                let args: GetTurnsArgs = match serde_json::from_value(arguments) {
                    Ok(args) => args,
                    Err(e) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(Self::parse_validation_error("get_turns", e)),
                        };
                    }
                };
                handle_get_turns(&self.client, args).await
            }
            _ => Err(format!("Unknown tool: {}", tool_name)),
        };

        match result {
            Ok(content) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": serde_json::to_string(&content).unwrap_or_else(|_| content.to_string())
                        }
                    ]
                })),
                error: None,
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: e,
                    data: None,
                }),
            },
        }
    }
}

/// Run the MCP server over stdio.
pub async fn run_server(client: Client) -> anyhow::Result<()> {
    let server = AgTraceServer::new(client);
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let reader = BufReader::new(stdin);

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(req) => req,
            Err(e) => {
                // For parse errors, we can't get a valid id, so we use a sentinel value
                let error_response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Number(serde_json::Number::from(-1)),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                };
                let response_json = serde_json::to_string(&error_response)?;
                writeln!(stdout, "{}", response_json)?;
                stdout.flush()?;
                continue;
            }
        };

        let response = server.handle_request(request).await;
        let response_json = serde_json::to_string(&response)?;
        writeln!(stdout, "{}", response_json)?;
        stdout.flush()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {}
