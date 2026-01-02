use agtrace_sdk::Client;
use schemars::schema_for;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};

use super::dto::{
    AnalyzeSessionArgs, GetEventDetailsArgs, GetSessionDetailsArgs, ListSessionsArgs,
    SearchEventPreviewsArgs,
};
use super::tools::{
    handle_analyze_session, handle_get_event_details, handle_get_project_info,
    handle_get_session_details, handle_list_sessions, handle_search_event_previews,
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
    /// According to MCP spec 2024-11-05, missing required parameters should return:
    /// - code: -32602 (Invalid params)
    /// - message: "Invalid params: ..." format
    /// - data: structured information about missing fields
    fn parse_validation_error(tool_name: &str, error: serde_json::Error) -> JsonRpcError {
        let error_msg = error.to_string();

        // Check if it's a "missing field" error
        // Format: "missing field `field_name`" or "missing field \"field_name\""
        if error_msg.contains("missing field")
            && let Some(field_start) = error_msg.find('`')
            && let Some(field_end) = error_msg[field_start + 1..].find('`')
        {
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
                "instructions": "AgTrace MCP Server - AI-native observability for agent execution traces. Use these tools to query historical sessions, analyze failures, search event payloads, and debug agent behavior."
            })),
            error: None,
        }
    }

    async fn handle_list_tools(&self, id: Value) -> JsonRpcResponse {
        // Generate JSON Schemas from Rust types - single source of truth!
        let list_sessions_schema = schema_for!(ListSessionsArgs);
        let get_session_details_schema = schema_for!(GetSessionDetailsArgs);
        let analyze_session_schema = schema_for!(AnalyzeSessionArgs);
        let search_event_previews_schema = schema_for!(SearchEventPreviewsArgs);
        let get_event_details_schema = schema_for!(GetEventDetailsArgs);

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "tools": [
                    {
                        "name": "list_sessions",
                        "description": "List recent AI agent sessions with cursor-based pagination",
                        "inputSchema": serde_json::to_value(&list_sessions_schema).unwrap(),
                    },
                    {
                        "name": "get_session_details",
                        "description": "Get session details with configurable verbosity (summary/turns/steps/full)",
                        "inputSchema": serde_json::to_value(&get_session_details_schema).unwrap(),
                    },
                    {
                        "name": "analyze_session",
                        "description": "Run diagnostic analysis on a session to identify failures, loops, and issues",
                        "inputSchema": serde_json::to_value(&analyze_session_schema).unwrap(),
                    },
                    {
                        "name": "search_event_previews",
                        "description": "Search for patterns in event payloads across sessions (returns previews, ~300 char snippets)",
                        "inputSchema": serde_json::to_value(&search_event_previews_schema).unwrap(),
                    },
                    {
                        "name": "get_event_details",
                        "description": "Retrieve full event payload by session ID and event index",
                        "inputSchema": serde_json::to_value(&get_event_details_schema).unwrap(),
                    },
                    {
                        "name": "get_project_info",
                        "description": "List all projects that have been indexed by agtrace with their metadata",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
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
            "get_session_details" => {
                let args: GetSessionDetailsArgs = match serde_json::from_value(arguments) {
                    Ok(args) => args,
                    Err(e) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(Self::parse_validation_error("get_session_details", e)),
                        };
                    }
                };
                handle_get_session_details(&self.client, args).await
            }
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
            "search_event_previews" => {
                let args: SearchEventPreviewsArgs = match serde_json::from_value(arguments) {
                    Ok(args) => args,
                    Err(e) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(Self::parse_validation_error("search_event_previews", e)),
                        };
                    }
                };
                handle_search_event_previews(&self.client, args).await
            }
            "get_event_details" => {
                let args: GetEventDetailsArgs = match serde_json::from_value(arguments) {
                    Ok(args) => args,
                    Err(e) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(Self::parse_validation_error("get_event_details", e)),
                        };
                    }
                };
                handle_get_event_details(&self.client, args).await
            }
            "get_project_info" => handle_get_project_info(&self.client).await,
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
                            "text": serde_json::to_string_pretty(&content).unwrap_or_else(|_| content.to_string())
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
mod tests {
    use super::*;

    #[test]
    fn test_parse_validation_error_missing_field() {
        // Simulate serde error for missing field
        let json_str = r#"{}"#;
        let result: Result<super::super::dto::GetSessionDetailsArgs, _> =
            serde_json::from_str(json_str);

        let error = result.unwrap_err();
        let json_error = AgTraceServer::parse_validation_error("get_session_details", error);

        assert_eq!(json_error.code, -32602);
        assert!(
            json_error.message.starts_with("Invalid params:"),
            "Message should start with 'Invalid params:'"
        );
        assert!(
            json_error.message.contains("session_id"),
            "Message should mention the missing field"
        );

        // Verify data field structure
        let data = json_error.data.expect("data field should be present");
        assert_eq!(data["tool"], "get_session_details");
        assert!(
            data["missing"].is_array(),
            "missing field should be an array"
        );
        let missing = data["missing"].as_array().unwrap();
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], "session_id");
    }

    #[test]
    fn test_parse_validation_error_other_errors() {
        // Simulate serde error for invalid type
        let json_str = r#"{"session_id": 123}"#; // number instead of string
        let result: Result<super::super::dto::GetSessionDetailsArgs, _> =
            serde_json::from_str(json_str);

        let error = result.unwrap_err();
        let json_error = AgTraceServer::parse_validation_error("get_session_details", error);

        assert_eq!(json_error.code, -32602);
        assert!(
            json_error.message.starts_with("Invalid params:"),
            "Message should start with 'Invalid params:'"
        );

        // For non-missing-field errors, should have detail field
        let data = json_error.data.expect("data field should be present");
        assert_eq!(data["tool"], "get_session_details");
        assert!(
            data["detail"].is_string(),
            "detail field should contain error message"
        );
    }
}
