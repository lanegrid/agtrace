use agtrace_types::{ToolCallPayload, ToolKind, ToolOrigin};
use serde_json::Value;

use crate::gemini::tools::{
    GeminiGoogleWebSearchArgs, GeminiReadFileArgs, GeminiReplaceArgs, GeminiRunShellCommandArgs,
    GeminiWriteFileArgs, GeminiWriteTodosArgs,
};

/// Normalize Gemini-specific tool calls
///
/// Handles Gemini provider-specific tool names and maps them to domain variants.
/// Uses provider-specific Args structs for proper schema parsing and conversion.
///
/// # MCP Tool Detection
///
/// Gemini uses a different naming convention for MCP tools compared to other providers:
/// - Tool name: "o3-search" (no mcp__ prefix)
/// - Display name: "o3-search (o3 MCP Server)"
///
/// We detect MCP tools by checking if display_name contains "MCP Server" pattern.
pub(crate) fn normalize_gemini_tool_call(
    tool_name: String,
    arguments: serde_json::Value,
    provider_call_id: Option<String>,
    display_name: Option<String>,
) -> ToolCallPayload {
    // Handle Gemini-specific tools with provider-specific types
    match tool_name.as_str() {
        "read_file" => {
            // Parse as Gemini-specific Args, then convert to domain model
            if let Ok(gemini_args) = serde_json::from_value::<GeminiReadFileArgs>(arguments.clone())
            {
                return ToolCallPayload::FileRead {
                    name: tool_name,
                    arguments: gemini_args.to_file_read_args(),
                    provider_call_id,
                };
            }
        }
        "write_file" => {
            // Parse as Gemini-specific Args, then convert to domain model
            if let Ok(gemini_args) =
                serde_json::from_value::<GeminiWriteFileArgs>(arguments.clone())
            {
                return ToolCallPayload::FileWrite {
                    name: tool_name,
                    arguments: gemini_args.to_file_write_args(),
                    provider_call_id,
                };
            }
        }
        "replace" => {
            // Parse as Gemini-specific Args (with instruction field), then convert
            if let Ok(gemini_args) = serde_json::from_value::<GeminiReplaceArgs>(arguments.clone())
            {
                return ToolCallPayload::FileEdit {
                    name: tool_name,
                    arguments: gemini_args.to_file_edit_args(),
                    provider_call_id,
                };
            }
        }
        "run_shell_command" => {
            // Parse as Gemini-specific Args, then convert to domain model
            if let Ok(gemini_args) =
                serde_json::from_value::<GeminiRunShellCommandArgs>(arguments.clone())
            {
                return ToolCallPayload::Execute {
                    name: tool_name,
                    arguments: gemini_args.to_execute_args(),
                    provider_call_id,
                };
            }
        }
        "google_web_search" => {
            // Parse as Gemini-specific Args, then convert to domain model
            if let Ok(gemini_args) =
                serde_json::from_value::<GeminiGoogleWebSearchArgs>(arguments.clone())
            {
                return ToolCallPayload::Search {
                    name: tool_name,
                    arguments: gemini_args.to_search_args(),
                    provider_call_id,
                };
            }
        }
        "write_todos" => {
            // Validate as Gemini-specific Args, then keep as Generic
            // (no unified Plan variant exists yet in domain model)
            if serde_json::from_value::<GeminiWriteTodosArgs>(arguments.clone()).is_ok() {
                return ToolCallPayload::Generic {
                    name: tool_name,
                    arguments,
                    provider_call_id,
                };
            }
        }
        _ => {
            // Check if this is an MCP tool using display name
            if super::tool_mapping::is_mcp_tool(display_name.as_deref()) {
                let server = super::tool_mapping::extract_mcp_server_name(display_name.as_deref());
                let tool = Some(tool_name.clone());

                if let Ok(mut args) =
                    serde_json::from_value::<agtrace_types::McpArgs>(arguments.clone())
                {
                    args.server = server;
                    args.tool = tool;

                    return ToolCallPayload::Mcp {
                        name: tool_name,
                        arguments: args,
                        provider_call_id,
                    };
                }
            }

            // Unknown Gemini tool, fall through to Generic
        }
    }

    // Fallback to generic
    ToolCallPayload::Generic {
        name: tool_name,
        arguments,
        provider_call_id,
    }
}

/// Gemini tool mapper implementation
pub struct GeminiToolMapper;

impl crate::traits::ToolMapper for GeminiToolMapper {
    fn classify(&self, tool_name: &str) -> (ToolOrigin, ToolKind) {
        super::tool_mapping::classify_tool(tool_name)
            .unwrap_or_else(|| crate::tool_analyzer::classify_common(tool_name))
    }

    fn normalize_call(&self, name: &str, args: Value, call_id: Option<String>) -> ToolCallPayload {
        // Note: ToolMapper trait doesn't have display_name parameter,
        // so we pass None here. The actual display_name is passed from parser.
        normalize_gemini_tool_call(name.to_string(), args, call_id, None)
    }

    fn summarize(&self, kind: ToolKind, args: &Value) -> String {
        crate::tool_analyzer::extract_common_summary(kind, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_read_file() {
        let payload = normalize_gemini_tool_call(
            "read_file".to_string(),
            serde_json::json!({"file_path": "src/main.rs"}),
            Some("call_123".to_string()),
            None,
        );

        match payload {
            ToolCallPayload::FileRead {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "read_file");
                assert_eq!(arguments.file_path, Some("src/main.rs".to_string()));
                assert_eq!(provider_call_id, Some("call_123".to_string()));
            }
            _ => panic!("Expected FileRead variant"),
        }
    }

    #[test]
    fn test_normalize_write_file() {
        let payload = normalize_gemini_tool_call(
            "write_file".to_string(),
            serde_json::json!({"file_path": "test.txt", "content": "hello"}),
            Some("call_456".to_string()),
            None,
        );

        match payload {
            ToolCallPayload::FileWrite {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "write_file");
                assert_eq!(arguments.file_path, "test.txt");
                assert_eq!(arguments.content, "hello");
                assert_eq!(provider_call_id, Some("call_456".to_string()));
            }
            _ => panic!("Expected FileWrite variant"),
        }
    }

    #[test]
    fn test_normalize_run_shell_command() {
        let payload = normalize_gemini_tool_call(
            "run_shell_command".to_string(),
            serde_json::json!({"command": "ls -la"}),
            Some("call_789".to_string()),
            None,
        );

        match payload {
            ToolCallPayload::Execute {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "run_shell_command");
                assert_eq!(arguments.command, Some("ls -la".to_string()));
                assert_eq!(provider_call_id, Some("call_789".to_string()));
            }
            _ => panic!("Expected Execute variant"),
        }
    }

    #[test]
    fn test_normalize_replace() {
        let payload = normalize_gemini_tool_call(
            "replace".to_string(),
            serde_json::json!({
                "file_path": "test.txt",
                "old_string": "old",
                "new_string": "new"
            }),
            None,
            None,
        );

        match payload {
            ToolCallPayload::FileEdit {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "replace");
                assert_eq!(arguments.file_path, "test.txt");
                assert_eq!(arguments.old_string, "old");
                assert_eq!(arguments.new_string, "new");
                assert_eq!(provider_call_id, None);
            }
            _ => panic!("Expected FileEdit variant"),
        }
    }

    #[test]
    fn test_normalize_unknown_gemini_tool() {
        let payload = normalize_gemini_tool_call(
            "unknown_tool".to_string(),
            serde_json::json!({"arg": "value"}),
            None,
            None,
        );

        match payload {
            ToolCallPayload::Generic {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "unknown_tool");
                assert_eq!(arguments["arg"], "value");
                assert_eq!(provider_call_id, None);
            }
            _ => panic!("Expected Generic variant"),
        }
    }

    #[test]
    fn test_normalize_mcp_tool_with_display_name() {
        let payload = normalize_gemini_tool_call(
            "o3-search".to_string(),
            serde_json::json!({"input": "ice coffee taste"}),
            Some("call_mcp".to_string()),
            Some("o3-search (o3 MCP Server)".to_string()),
        );

        match payload {
            ToolCallPayload::Mcp {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "o3-search");
                assert_eq!(arguments.server, Some("o3".to_string()));
                assert_eq!(arguments.tool, Some("o3-search".to_string()));
                assert_eq!(
                    arguments.inner.get("input").and_then(|v| v.as_str()),
                    Some("ice coffee taste")
                );
                assert_eq!(provider_call_id, Some("call_mcp".to_string()));
            }
            _ => panic!("Expected Mcp variant, got: {:?}", payload.kind()),
        }
    }

    #[test]
    fn test_normalize_non_mcp_tool_with_non_mcp_display_name() {
        let payload = normalize_gemini_tool_call(
            "google_web_search".to_string(),
            serde_json::json!({"query": "rust programming"}),
            Some("call_123".to_string()),
            Some("Google Web Search".to_string()),
        );

        match payload {
            ToolCallPayload::Search { name, .. } => {
                assert_eq!(name, "google_web_search");
            }
            _ => panic!("Expected Search variant, got: {:?}", payload.kind()),
        }
    }
}
