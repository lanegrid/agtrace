use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::args::{ExecuteArgs, FileEditArgs, FileReadArgs, FileWriteArgs, McpArgs, SearchArgs};
use super::kind::ToolKind;

/// Normalized tool call with structured arguments
///
/// This enum provides type-safe access to common tool call patterns while
/// maintaining compatibility with the original JSON structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolCallPayload {
    /// File read operation (Read, Glob, etc.)
    FileRead {
        name: String,
        arguments: FileReadArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_call_id: Option<String>,
    },

    /// File edit operation (Edit)
    FileEdit {
        name: String,
        arguments: FileEditArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_call_id: Option<String>,
    },

    /// File write operation (Write)
    FileWrite {
        name: String,
        arguments: FileWriteArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_call_id: Option<String>,
    },

    /// Execute/shell command (Bash, etc.)
    Execute {
        name: String,
        arguments: ExecuteArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_call_id: Option<String>,
    },

    /// Search operation (Grep, WebSearch, etc.)
    Search {
        name: String,
        arguments: SearchArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_call_id: Option<String>,
    },

    /// MCP (Model Context Protocol) tool call
    Mcp {
        name: String,
        arguments: McpArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_call_id: Option<String>,
    },

    /// Generic/fallback for unknown or custom tools
    Generic {
        name: String,
        arguments: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_call_id: Option<String>,
    },
}

impl ToolCallPayload {
    /// Get tool name regardless of variant
    pub fn name(&self) -> &str {
        match self {
            ToolCallPayload::FileRead { name, .. } => name,
            ToolCallPayload::FileEdit { name, .. } => name,
            ToolCallPayload::FileWrite { name, .. } => name,
            ToolCallPayload::Execute { name, .. } => name,
            ToolCallPayload::Search { name, .. } => name,
            ToolCallPayload::Mcp { name, .. } => name,
            ToolCallPayload::Generic { name, .. } => name,
        }
    }

    /// Get provider call ID regardless of variant
    pub fn provider_call_id(&self) -> Option<&str> {
        match self {
            ToolCallPayload::FileRead {
                provider_call_id, ..
            } => provider_call_id.as_deref(),
            ToolCallPayload::FileEdit {
                provider_call_id, ..
            } => provider_call_id.as_deref(),
            ToolCallPayload::FileWrite {
                provider_call_id, ..
            } => provider_call_id.as_deref(),
            ToolCallPayload::Execute {
                provider_call_id, ..
            } => provider_call_id.as_deref(),
            ToolCallPayload::Search {
                provider_call_id, ..
            } => provider_call_id.as_deref(),
            ToolCallPayload::Mcp {
                provider_call_id, ..
            } => provider_call_id.as_deref(),
            ToolCallPayload::Generic {
                provider_call_id, ..
            } => provider_call_id.as_deref(),
        }
    }

    /// Derive semantic ToolKind from ToolCallPayload variant
    pub fn kind(&self) -> ToolKind {
        match self {
            ToolCallPayload::FileRead { .. } => ToolKind::Read,
            ToolCallPayload::FileEdit { .. } => ToolKind::Write,
            ToolCallPayload::FileWrite { .. } => ToolKind::Write,
            ToolCallPayload::Execute { .. } => ToolKind::Execute,
            ToolCallPayload::Search { .. } => ToolKind::Search,
            ToolCallPayload::Mcp { .. } => ToolKind::Other,
            ToolCallPayload::Generic { .. } => ToolKind::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_call_serialization_roundtrip() {
        let original = ToolCallPayload::FileRead {
            name: "Read".to_string(),
            arguments: FileReadArgs {
                file_path: Some("/path/to/file.rs".to_string()),
                path: None,
                pattern: None,
                extra: serde_json::json!({}),
            },
            provider_call_id: Some("call_123".to_string()),
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ToolCallPayload = serde_json::from_str(&json).unwrap();

        match deserialized {
            ToolCallPayload::FileRead {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "Read");
                assert_eq!(arguments.file_path, Some("/path/to/file.rs".to_string()));
                assert_eq!(provider_call_id, Some("call_123".to_string()));
            }
            _ => panic!("Expected FileRead variant"),
        }
    }

    #[test]
    fn test_tool_call_kind_derivation() {
        let read_payload = ToolCallPayload::FileRead {
            name: "Read".to_string(),
            arguments: FileReadArgs {
                file_path: Some("/path".to_string()),
                path: None,
                pattern: None,
                extra: serde_json::json!({}),
            },
            provider_call_id: None,
        };
        assert_eq!(read_payload.kind(), ToolKind::Read);

        let edit_payload = ToolCallPayload::FileEdit {
            name: "Edit".to_string(),
            arguments: FileEditArgs {
                file_path: "/path".to_string(),
                old_string: "old".to_string(),
                new_string: "new".to_string(),
                replace_all: false,
            },
            provider_call_id: None,
        };
        assert_eq!(edit_payload.kind(), ToolKind::Write);

        let write_payload = ToolCallPayload::FileWrite {
            name: "Write".to_string(),
            arguments: FileWriteArgs {
                file_path: "/path".to_string(),
                content: "content".to_string(),
            },
            provider_call_id: None,
        };
        assert_eq!(write_payload.kind(), ToolKind::Write);

        let exec_payload = ToolCallPayload::Execute {
            name: "Bash".to_string(),
            arguments: ExecuteArgs {
                command: Some("ls".to_string()),
                description: None,
                timeout: None,
                extra: serde_json::json!({}),
            },
            provider_call_id: None,
        };
        assert_eq!(exec_payload.kind(), ToolKind::Execute);

        let search_payload = ToolCallPayload::Search {
            name: "Grep".to_string(),
            arguments: SearchArgs {
                pattern: Some("pattern".to_string()),
                query: None,
                input: None,
                path: None,
                extra: serde_json::json!({}),
            },
            provider_call_id: None,
        };
        assert_eq!(search_payload.kind(), ToolKind::Search);

        let mcp_payload = ToolCallPayload::Mcp {
            name: "mcp__o3__search".to_string(),
            arguments: McpArgs {
                server: Some("o3".to_string()),
                tool: Some("search".to_string()),
                inner: serde_json::json!({}),
            },
            provider_call_id: None,
        };
        assert_eq!(mcp_payload.kind(), ToolKind::Other);

        let generic_payload = ToolCallPayload::Generic {
            name: "CustomTool".to_string(),
            arguments: serde_json::json!({}),
            provider_call_id: None,
        };
        assert_eq!(generic_payload.kind(), ToolKind::Other);
    }
}
