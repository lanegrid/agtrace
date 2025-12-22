use agtrace_types::{ToolCallPayload, ToolKind, ToolOrigin};
use serde_json::Value;

use crate::codex::tools::{ApplyPatchArgs, PatchOperation, ReadMcpResourceArgs, ShellArgs};
use agtrace_types::{ExecuteArgs, FileEditArgs, FileWriteArgs};

/// Normalize Codex-specific tool calls
///
/// Handles provider-specific tools like apply_patch before falling back to generic normalization.
pub(crate) fn normalize_codex_tool_call(
    tool_name: String,
    arguments: serde_json::Value,
    provider_call_id: Option<String>,
) -> ToolCallPayload {
    // Handle Codex-specific tools
    match tool_name.as_str() {
        "apply_patch" => {
            // Try to parse as ApplyPatchArgs
            if let Ok(patch_args) = serde_json::from_value::<ApplyPatchArgs>(arguments.clone()) {
                // Parse the patch structure
                match patch_args.parse() {
                    Ok(parsed) => {
                        // Map to FileWrite (Add) or FileEdit (Update) based on operation
                        match parsed.operation {
                            PatchOperation::Add => {
                                // New file creation → FileWrite
                                return ToolCallPayload::FileWrite {
                                    name: tool_name,
                                    arguments: FileWriteArgs {
                                        file_path: parsed.file_path,
                                        content: parsed.raw_patch,
                                    },
                                    provider_call_id,
                                };
                            }
                            PatchOperation::Update => {
                                // File modification → FileEdit
                                // Note: For patches, we store the raw patch in old_string/new_string
                                // as a placeholder. The actual diff is in the raw patch.
                                return ToolCallPayload::FileEdit {
                                    name: tool_name,
                                    arguments: FileEditArgs {
                                        file_path: parsed.file_path,
                                        old_string: String::new(), // Placeholder: actual diff in raw patch
                                        new_string: parsed.raw_patch.clone(),
                                        replace_all: false,
                                    },
                                    provider_call_id,
                                };
                            }
                        }
                    }
                    Err(_) => {
                        // Parsing failed, fall back to generic
                    }
                }
            }
        }
        "shell" => {
            // Try to parse as ShellArgs
            if let Ok(shell_args) = serde_json::from_value::<ShellArgs>(arguments.clone()) {
                // Convert to standard ExecuteArgs
                let execute_args = shell_args.to_execute_args();
                return ToolCallPayload::Execute {
                    name: tool_name,
                    arguments: execute_args,
                    provider_call_id,
                };
            }
        }
        "read_mcp_resource" => {
            // Try to parse as ReadMcpResourceArgs
            if let Ok(mcp_args) = serde_json::from_value::<ReadMcpResourceArgs>(arguments.clone()) {
                // Convert to standard FileReadArgs
                let file_read_args = mcp_args.to_file_read_args();
                return ToolCallPayload::FileRead {
                    name: tool_name,
                    arguments: file_read_args,
                    provider_call_id,
                };
            }
        }
        "shell_command" => {
            // shell_command → Execute (already uses string command format)
            if let Ok(args) = serde_json::from_value::<ExecuteArgs>(arguments.clone()) {
                return ToolCallPayload::Execute {
                    name: tool_name,
                    arguments: args,
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
            // Unknown Codex tool, fall through to Generic
        }
    }

    // Fallback to generic
    ToolCallPayload::Generic {
        name: tool_name,
        arguments,
        provider_call_id,
    }
}

/// Codex tool mapper implementation
pub struct CodexToolMapper;

impl crate::traits::ToolMapper for CodexToolMapper {
    fn classify(&self, tool_name: &str) -> (ToolOrigin, ToolKind) {
        super::tool_mapping::classify_tool(tool_name)
            .unwrap_or_else(|| crate::tool_analyzer::classify_common(tool_name))
    }

    fn normalize_call(&self, name: &str, args: Value, call_id: Option<String>) -> ToolCallPayload {
        normalize_codex_tool_call(name.to_string(), args, call_id)
    }

    fn summarize(&self, kind: ToolKind, args: &Value) -> String {
        crate::tool_analyzer::extract_common_summary(kind, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_apply_patch_update_file() {
        let raw_patch = r#"*** Begin Patch
*** Update File: test.rs
@@
-old line
+new line
@@
*** End Patch"#;

        let arguments = serde_json::json!({ "raw": raw_patch });
        let payload = normalize_codex_tool_call(
            "apply_patch".to_string(),
            arguments,
            Some("call_456".to_string()),
        );

        match payload {
            ToolCallPayload::FileEdit {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "apply_patch");
                assert_eq!(arguments.file_path, "test.rs");
                assert!(arguments.new_string.contains("*** Begin Patch"));
                assert_eq!(provider_call_id, Some("call_456".to_string()));
            }
            _ => panic!("Expected FileEdit variant"),
        }
    }

    #[test]
    fn test_normalize_apply_patch_add_file() {
        let raw_patch = r#"*** Begin Patch
*** Add File: newfile.txt
@@
+new content
@@
*** End Patch"#;

        let arguments = serde_json::json!({ "raw": raw_patch });
        let payload = normalize_codex_tool_call(
            "apply_patch".to_string(),
            arguments,
            Some("call_789".to_string()),
        );

        match payload {
            ToolCallPayload::FileWrite {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "apply_patch");
                assert_eq!(arguments.file_path, "newfile.txt");
                assert!(arguments.content.contains("*** Begin Patch"));
                assert_eq!(provider_call_id, Some("call_789".to_string()));
            }
            _ => panic!("Expected FileWrite variant"),
        }
    }

    #[test]
    fn test_normalize_shell_command() {
        let arguments = serde_json::json!({
            "command": ["ls", "-la"],
            "cwd": "/home/user",
            "description": "List files"
        });

        let payload =
            normalize_codex_tool_call("shell".to_string(), arguments, Some("call_123".to_string()));

        match payload {
            ToolCallPayload::Execute {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "shell");
                assert_eq!(arguments.command, Some("ls -la".to_string()));
                assert_eq!(provider_call_id, Some("call_123".to_string()));
            }
            _ => panic!("Expected Execute variant"),
        }
    }

    #[test]
    fn test_normalize_shell_minimal() {
        let arguments = serde_json::json!({
            "command": ["echo", "hello"]
        });

        let payload = normalize_codex_tool_call("shell".to_string(), arguments, None);

        match payload {
            ToolCallPayload::Execute {
                name, arguments, ..
            } => {
                assert_eq!(name, "shell");
                assert_eq!(arguments.command, Some("echo hello".to_string()));
            }
            _ => panic!("Expected Execute variant"),
        }
    }

    #[test]
    fn test_normalize_shell_with_all_fields() {
        let arguments = serde_json::json!({
            "command": ["python", "script.py"],
            "cwd": "/workspace",
            "description": "Run Python script",
            "timeout_ms": 5000
        });

        let payload = normalize_codex_tool_call("shell".to_string(), arguments, None);

        match payload {
            ToolCallPayload::Execute {
                name, arguments, ..
            } => {
                assert_eq!(name, "shell");
                assert_eq!(arguments.command, Some("python script.py".to_string()));
            }
            _ => panic!("Expected Execute variant"),
        }
    }

    #[test]
    fn test_normalize_read_mcp_resource() {
        let arguments = serde_json::json!({
            "server": "local",
            "uri": "file:///path/to/file.txt"
        });

        let payload = normalize_codex_tool_call(
            "read_mcp_resource".to_string(),
            arguments,
            Some("call_999".to_string()),
        );

        match payload {
            ToolCallPayload::FileRead {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "read_mcp_resource");
                assert_eq!(
                    arguments.file_path,
                    Some("file:///path/to/file.txt".to_string())
                );
                assert_eq!(provider_call_id, Some("call_999".to_string()));
            }
            _ => panic!("Expected FileRead variant"),
        }
    }
}
