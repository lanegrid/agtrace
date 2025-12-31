use agtrace_types::{ToolCallPayload, ToolKind, ToolOrigin};
use serde_json::Value;

use crate::codex::tools::{ApplyPatchArgs, PatchOperation, ReadMcpResourceArgs, ShellArgs};
use agtrace_types::{ExecuteArgs, FileEditArgs, FileReadArgs, FileWriteArgs};

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

                // Check if this is a read-oriented command
                if let Some(command) = &execute_args.command {
                    if super::execute_intent::classify_execute_command(command) == Some(ToolKind::Read) {
                        // Extract file path if possible
                        let file_path = super::execute_intent::extract_file_path(command);

                        return ToolCallPayload::FileRead {
                            name: tool_name,
                            arguments: FileReadArgs {
                                file_path,
                                path: None,
                                pattern: None,
                                extra: serde_json::json!({"command": command}),
                            },
                            provider_call_id,
                        };
                    }
                }

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
                // Check if this is a read-oriented command
                if let Some(command) = &args.command {
                    if super::execute_intent::classify_execute_command(command) == Some(ToolKind::Read) {
                        // Extract file path if possible
                        let file_path = super::execute_intent::extract_file_path(command);

                        return ToolCallPayload::FileRead {
                            name: tool_name,
                            arguments: FileReadArgs {
                                file_path,
                                path: None,
                                pattern: None,
                                extra: serde_json::json!({"command": command}),
                            },
                            provider_call_id,
                        };
                    }
                }

                return ToolCallPayload::Execute {
                    name: tool_name,
                    arguments: args,
                    provider_call_id,
                };
            }
        }
        _ if tool_name.starts_with("mcp__") => {
            // MCP tools - parse server and tool names using Codex-specific convention
            let (server, tool) = super::tool_mapping::parse_mcp_name(&tool_name)
                .map(|(s, t)| (Some(s), Some(t)))
                .unwrap_or((None, None));

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
        // NOTE: ls is now classified as Read (file listing)
        let arguments = serde_json::json!({
            "command": ["ls", "-la"],
            "cwd": "/home/user",
            "description": "List files"
        });

        let payload =
            normalize_codex_tool_call("shell".to_string(), arguments, Some("call_123".to_string()));

        match payload {
            ToolCallPayload::FileRead {
                name,
                provider_call_id,
                ..
            } => {
                assert_eq!(name, "shell");
                assert_eq!(provider_call_id, Some("call_123".to_string()));
            }
            _ => panic!("Expected FileRead variant for ls command, got: {:?}", payload.kind()),
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

    #[test]
    fn test_normalize_mcp_tool_parses_server_and_tool() {
        let payload = normalize_codex_tool_call(
            "mcp__filesystem__read".to_string(),
            serde_json::json!({"path": "/tmp/file.txt"}),
            Some("call_mcp".to_string()),
        );

        match payload {
            ToolCallPayload::Mcp {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "mcp__filesystem__read");
                assert_eq!(arguments.server, Some("filesystem".to_string()));
                assert_eq!(arguments.tool, Some("read".to_string()));
                assert_eq!(
                    arguments.inner.get("path").and_then(|v| v.as_str()),
                    Some("/tmp/file.txt")
                );
                assert_eq!(provider_call_id, Some("call_mcp".to_string()));
            }
            _ => panic!("Expected Mcp variant"),
        }
    }

    #[test]
    fn test_normalize_mcp_tool_handles_malformed_name() {
        let payload = normalize_codex_tool_call(
            "mcp__noserver".to_string(),
            serde_json::json!({"data": "test"}),
            None,
        );

        match payload {
            ToolCallPayload::Mcp {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "mcp__noserver");
                // Malformed MCP name should result in None for both server and tool
                assert_eq!(arguments.server, None);
                assert_eq!(arguments.tool, None);
                assert_eq!(provider_call_id, None);
            }
            _ => panic!("Expected Mcp variant"),
        }
    }

    #[test]
    fn test_normalize_shell_read_command_cat() {
        let payload = normalize_codex_tool_call(
            "shell".to_string(),
            serde_json::json!({
                "command": ["cat", "file.txt"]
            }),
            Some("call_read".to_string()),
        );

        match payload {
            ToolCallPayload::FileRead {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "shell");
                assert_eq!(arguments.file_path, Some("file.txt".to_string()));
                assert_eq!(provider_call_id, Some("call_read".to_string()));
            }
            _ => panic!("Expected FileRead variant for cat command, got: {:?}", payload.kind()),
        }
    }

    #[test]
    fn test_normalize_shell_read_command_sed() {
        let payload = normalize_codex_tool_call(
            "shell".to_string(),
            serde_json::json!({
                "command": ["sed", "-n", "1,200p", "packages/extension-inspector/src/App.tsx"]
            }),
            None,
        );

        match payload {
            ToolCallPayload::FileRead {
                name, arguments, ..
            } => {
                assert_eq!(name, "shell");
                assert_eq!(
                    arguments.file_path,
                    Some("packages/extension-inspector/src/App.tsx".to_string())
                );
            }
            _ => panic!("Expected FileRead variant for sed -n command, got: {:?}", payload.kind()),
        }
    }

    #[test]
    fn test_normalize_shell_read_command_ls() {
        let payload = normalize_codex_tool_call(
            "shell".to_string(),
            serde_json::json!({
                "command": ["ls", "-la"]
            }),
            None,
        );

        match payload {
            ToolCallPayload::FileRead { name, .. } => {
                assert_eq!(name, "shell");
                // ls doesn't have a specific file, so file_path is None
            }
            _ => panic!("Expected FileRead variant for ls command, got: {:?}", payload.kind()),
        }
    }

    #[test]
    fn test_normalize_shell_write_command_mkdir() {
        let payload = normalize_codex_tool_call(
            "shell".to_string(),
            serde_json::json!({
                "command": ["mkdir", "-p", "mydir"]
            }),
            None,
        );

        match payload {
            ToolCallPayload::Execute { name, .. } => {
                assert_eq!(name, "shell");
                // mkdir is a write command, should remain Execute
            }
            _ => panic!("Expected Execute variant for mkdir command, got: {:?}", payload.kind()),
        }
    }

    #[test]
    fn test_normalize_shell_command_read() {
        let payload = normalize_codex_tool_call(
            "shell_command".to_string(),
            serde_json::json!({
                "command": "grep pattern file.txt"
            }),
            None,
        );

        match payload {
            ToolCallPayload::FileRead {
                name, arguments, ..
            } => {
                assert_eq!(name, "shell_command");
                assert_eq!(arguments.file_path, Some("file.txt".to_string()));
            }
            _ => panic!("Expected FileRead variant for grep command, got: {:?}", payload.kind()),
        }
    }

    #[test]
    fn test_normalize_shell_bash_wrapped_read() {
        let payload = normalize_codex_tool_call(
            "shell".to_string(),
            serde_json::json!({
                "command": ["bash", "-lc", "cat", "Cargo.toml"]
            }),
            None,
        );

        match payload {
            ToolCallPayload::FileRead {
                name, arguments, ..
            } => {
                assert_eq!(name, "shell");
                assert_eq!(arguments.file_path, Some("Cargo.toml".to_string()));
            }
            _ => panic!("Expected FileRead variant for bash-wrapped cat, got: {:?}", payload.kind()),
        }
    }
}
