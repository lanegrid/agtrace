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
                            error: Some(JsonRpcError {
                                code: -32602,
                                message: format!("Invalid arguments: {}", e),
                                data: None,
                            }),
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
                            error: Some(JsonRpcError {
                                code: -32602,
                                message: format!("Invalid arguments: {}", e),
                                data: None,
                            }),
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
                            error: Some(JsonRpcError {
                                code: -32602,
                                message: format!("Invalid arguments: {}", e),
                                data: None,
                            }),
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
                            error: Some(JsonRpcError {
                                code: -32602,
                                message: format!("Invalid arguments: {}", e),
                                data: None,
                            }),
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
                            error: Some(JsonRpcError {
                                code: -32602,
                                message: format!("Invalid arguments: {}", e),
                                data: None,
                            }),
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
