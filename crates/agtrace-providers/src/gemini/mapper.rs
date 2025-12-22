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
pub(crate) fn normalize_gemini_tool_call(
    tool_name: String,
    arguments: serde_json::Value,
    provider_call_id: Option<String>,
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
        _ if tool_name.starts_with("mcp__") => {
            // MCP tools
            if let Ok(args) = serde_json::from_value(arguments.clone()) {
                return ToolCallPayload::Mcp {
                    name: tool_name,
                    arguments: args,
                    provider_call_id,
                };
            }
        }
        _ => {
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
        normalize_gemini_tool_call(name.to_string(), args, call_id)
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
}
