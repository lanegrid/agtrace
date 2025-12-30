use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tool classification by semantic purpose
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    /// Read operations (files, resources, data)
    Read,
    /// Write operations (edit, create, patch)
    Write,
    /// Execute operations (shell commands, scripts)
    Execute,
    /// Planning operations (todo, task management)
    Plan,
    /// Search operations (web, file search, grep)
    Search,
    /// User interaction (questions, prompts)
    Ask,
    /// Other/unknown operations
    Other,
}

/// Tool origin classification
///
/// Distinguishes between provider-native tools and MCP protocol tools.
///
/// # Important
/// The origin is determined by how the tool is invoked, not by what it operates on:
/// - `System`: Tool is built-in to the provider and invoked directly by the LLM
/// - `Mcp`: Tool is invoked via MCP protocol (typically prefixed with `mcp__`)
///
/// # Examples
/// - `Bash` (Claude Code) → System (provider-native tool)
/// - `read_mcp_resource` (Codex) → System (provider-native tool that happens to read MCP resources)
/// - `mcp__sqlite__query` → Mcp (external tool invoked via MCP protocol)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolOrigin {
    /// System-provided tool (built-in to the provider)
    System,
    /// MCP (Model Context Protocol) tool invoked via MCP protocol
    Mcp,
}

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

// --- Tool Arguments ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReadArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(flatten)]
    pub extra: Value,
}

impl FileReadArgs {
    /// Get file path from various field names
    pub fn path(&self) -> Option<&str> {
        self.file_path.as_deref().or(self.path.as_deref())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEditArgs {
    pub file_path: String,
    pub old_string: String,
    pub new_string: String,
    #[serde(default)]
    pub replace_all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWriteArgs {
    pub file_path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(flatten)]
    pub extra: Value,
}

impl ExecuteArgs {
    pub fn command(&self) -> Option<&str> {
        self.command.as_deref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(flatten)]
    pub extra: Value,
}

impl SearchArgs {
    /// Get search pattern from various field names
    pub fn pattern(&self) -> Option<&str> {
        self.pattern
            .as_deref()
            .or(self.query.as_deref())
            .or(self.input.as_deref())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpArgs {
    #[serde(flatten)]
    pub inner: Value,
}

impl McpArgs {
    /// Parse MCP tool name from full name (e.g., "mcp__o3__o3-search" -> ("o3", "o3-search"))
    pub fn parse_name(full_name: &str) -> Option<(String, String)> {
        if !full_name.starts_with("mcp__") {
            return None;
        }

        let rest = &full_name[5..]; // Remove "mcp__"
        let parts: Vec<&str> = rest.splitn(2, "__").collect();

        if parts.len() == 2 {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }

    /// Get server name from full MCP tool name
    pub fn server_name(full_name: &str) -> Option<String> {
        Self::parse_name(full_name).map(|(server, _)| server)
    }

    /// Get tool name from full MCP tool name
    pub fn tool_name(full_name: &str) -> Option<String> {
        Self::parse_name(full_name).map(|(_, tool)| tool)
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
    fn test_file_read_args_path_helper() {
        let args1 = FileReadArgs {
            file_path: Some("/path1".to_string()),
            path: None,
            pattern: None,
            extra: serde_json::json!({}),
        };
        assert_eq!(args1.path(), Some("/path1"));

        let args2 = FileReadArgs {
            file_path: None,
            path: Some("/path2".to_string()),
            pattern: None,
            extra: serde_json::json!({}),
        };
        assert_eq!(args2.path(), Some("/path2"));

        let args3 = FileReadArgs {
            file_path: Some("/path1".to_string()),
            path: Some("/path2".to_string()),
            pattern: None,
            extra: serde_json::json!({}),
        };
        assert_eq!(args3.path(), Some("/path1"));
    }

    #[test]
    fn test_search_args_pattern_helper() {
        let args1 = SearchArgs {
            pattern: Some("pattern1".to_string()),
            query: None,
            input: None,
            path: None,
            extra: serde_json::json!({}),
        };
        assert_eq!(args1.pattern(), Some("pattern1"));

        let args2 = SearchArgs {
            pattern: None,
            query: Some("query2".to_string()),
            input: None,
            path: None,
            extra: serde_json::json!({}),
        };
        assert_eq!(args2.pattern(), Some("query2"));

        let args3 = SearchArgs {
            pattern: None,
            query: None,
            input: Some("input3".to_string()),
            path: None,
            extra: serde_json::json!({}),
        };
        assert_eq!(args3.pattern(), Some("input3"));
    }

    #[test]
    fn test_mcp_args_parse_name() {
        assert_eq!(
            McpArgs::parse_name("mcp__o3__o3-search"),
            Some(("o3".to_string(), "o3-search".to_string()))
        );

        assert_eq!(
            McpArgs::parse_name("mcp__sqlite__query"),
            Some(("sqlite".to_string(), "query".to_string()))
        );

        assert_eq!(McpArgs::parse_name("not_mcp_tool"), None);
        assert_eq!(McpArgs::parse_name("mcp__only_server"), None);
    }

    #[test]
    fn test_mcp_args_server_and_tool_name() {
        assert_eq!(
            McpArgs::server_name("mcp__o3__o3-search"),
            Some("o3".to_string())
        );
        assert_eq!(
            McpArgs::tool_name("mcp__o3__o3-search"),
            Some("o3-search".to_string())
        );
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
