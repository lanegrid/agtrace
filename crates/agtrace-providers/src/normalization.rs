// Tool call normalization from raw provider data to typed ToolCallPayload variants
//
// Rationale for provider-layer placement:
//   This module contains provider-specific knowledge about tool names and their
//   argument schemas. While the ToolCallPayload enum itself is in agtrace-types
//   (domain model), the logic to map raw tool names to typed variants belongs here.
//
// Design principle:
//   - agtrace-types: Defines domain model structure (ToolCallPayload enum)
//   - agtrace-providers: Knows how to normalize provider data into domain model
//   - This separation keeps types pure and provider logic centralized

use agtrace_types::ToolCallPayload;
use serde_json::Value;

/// Normalize raw tool call data into a typed ToolCallPayload variant
///
/// This function encapsulates provider-specific knowledge about:
/// - Tool name mapping (e.g., "Read" -> FileRead variant)
/// - Argument schema parsing (e.g., JSON -> FileReadArgs)
/// - Fallback handling (unknown tools -> Generic variant)
///
/// # Arguments
/// * `name` - Tool name from provider (e.g., "Read", "Bash", "mcp__o3__search")
/// * `arguments` - Raw JSON arguments from provider
/// * `provider_call_id` - Optional provider-specific call identifier
///
/// # Returns
/// Typed ToolCallPayload variant with parsed arguments, or Generic variant as fallback
pub fn normalize_tool_call(
    name: String,
    arguments: Value,
    provider_call_id: Option<String>,
) -> ToolCallPayload {
    // Try to parse into specific variants based on name
    match name.as_str() {
        "Read" | "Glob" => {
            if let Ok(args) = serde_json::from_value(arguments.clone()) {
                return ToolCallPayload::FileRead {
                    name,
                    arguments: args,
                    provider_call_id,
                };
            }
        }
        "Edit" => {
            if let Ok(args) = serde_json::from_value(arguments.clone()) {
                return ToolCallPayload::FileEdit {
                    name,
                    arguments: args,
                    provider_call_id,
                };
            }
        }
        "Write" => {
            if let Ok(args) = serde_json::from_value(arguments.clone()) {
                return ToolCallPayload::FileWrite {
                    name,
                    arguments: args,
                    provider_call_id,
                };
            }
        }
        "Bash" | "KillShell" | "BashOutput" => {
            if let Ok(args) = serde_json::from_value(arguments.clone()) {
                return ToolCallPayload::Execute {
                    name,
                    arguments: args,
                    provider_call_id,
                };
            }
        }
        "Grep" | "WebSearch" | "WebFetch" => {
            if let Ok(args) = serde_json::from_value(arguments.clone()) {
                return ToolCallPayload::Search {
                    name,
                    arguments: args,
                    provider_call_id,
                };
            }
        }
        _ if name.starts_with("mcp__") => {
            if let Ok(args) = serde_json::from_value(arguments.clone()) {
                return ToolCallPayload::Mcp {
                    name,
                    arguments: args,
                    provider_call_id,
                };
            }
        }
        _ => {}
    }

    // Fallback to generic
    ToolCallPayload::Generic {
        name,
        arguments,
        provider_call_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_normalize_file_read() {
        let payload = normalize_tool_call(
            "Read".to_string(),
            json!({"file_path": "/path/to/file.rs"}),
            Some("call_123".to_string()),
        );

        match payload {
            ToolCallPayload::FileRead {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "Read");
                assert_eq!(arguments.path(), Some("/path/to/file.rs"));
                assert_eq!(provider_call_id, Some("call_123".to_string()));
            }
            _ => panic!("Expected FileRead variant"),
        }
    }

    #[test]
    fn test_normalize_execute() {
        let payload = normalize_tool_call(
            "Bash".to_string(),
            json!({"command": "ls -la"}),
            Some("call_456".to_string()),
        );

        match payload {
            ToolCallPayload::Execute {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "Bash");
                assert_eq!(arguments.command, "ls -la");
                assert_eq!(provider_call_id, Some("call_456".to_string()));
            }
            _ => panic!("Expected Execute variant"),
        }
    }

    #[test]
    fn test_normalize_mcp_tool() {
        let payload = normalize_tool_call(
            "mcp__o3__search".to_string(),
            json!({"query": "test"}),
            Some("call_789".to_string()),
        );

        match payload {
            ToolCallPayload::Mcp {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "mcp__o3__search");
                assert_eq!(arguments.full_name(), "mcp__o3__search");
                assert_eq!(provider_call_id, Some("call_789".to_string()));
            }
            _ => panic!("Expected Mcp variant"),
        }
    }

    #[test]
    fn test_normalize_unknown_tool_fallback() {
        let payload = normalize_tool_call(
            "UnknownTool".to_string(),
            json!({"foo": "bar"}),
            Some("call_999".to_string()),
        );

        match payload {
            ToolCallPayload::Generic {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "UnknownTool");
                assert_eq!(arguments, json!({"foo": "bar"}));
                assert_eq!(provider_call_id, Some("call_999".to_string()));
            }
            _ => panic!("Expected Generic variant for unknown tool"),
        }
    }

    #[test]
    fn test_normalize_invalid_arguments_fallback() {
        // FileRead expects file_path, but we provide invalid args
        let payload = normalize_tool_call(
            "Read".to_string(),
            json!({"invalid_field": 123}),
            Some("call_000".to_string()),
        );

        // Should fall back to Generic when args don't match expected schema
        match payload {
            ToolCallPayload::Generic { name, .. } => {
                assert_eq!(name, "Read");
            }
            _ => panic!("Expected Generic variant for invalid arguments"),
        }
    }
}
