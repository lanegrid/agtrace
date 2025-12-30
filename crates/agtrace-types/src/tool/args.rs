use serde::{Deserialize, Serialize};
use serde_json::Value;

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
}
