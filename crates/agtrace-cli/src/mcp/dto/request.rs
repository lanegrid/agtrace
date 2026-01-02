// MCP Tool Request Types
//
// Design Notes:
// - All schemas auto-generated from Rust types via schemars (single source of truth)
// - DetailLevel: Progressive disclosure for get_session_details (summary → turns → steps → full)
// - SearchEventPreviewsArgs: Separated from deprecated SearchEventsArgs for clarity

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::{DetailLevel, EventType, Provider};

/// List recent AI agent sessions with cursor-based pagination
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListSessionsArgs {
    /// Maximum number of sessions to return (default: 10, max: 50)
    #[serde(default)]
    pub limit: Option<usize>,
    /// Pagination cursor from previous response's next_cursor field. Omit for first page.
    #[serde(default)]
    pub cursor: Option<String>,
    /// Filter by provider
    pub provider: Option<Provider>,
    /// Filter by project root path (e.g., "/Users/me/projects/my-app").
    /// Prefer this over project_hash when the agent knows the current working directory.
    /// Server will automatically resolve this to the correct project hash.
    pub project_root: Option<String>,
    /// Filter by project hash (internal ID).
    /// Use only when you have the exact hash; prefer project_root for ergonomic filtering.
    pub project_hash: Option<String>,
    /// Show sessions after this timestamp (ISO 8601)
    pub since: Option<String>,
    /// Show sessions before this timestamp (ISO 8601)
    pub until: Option<String>,
}

/// Get session details with configurable verbosity
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetSessionDetailsArgs {
    /// Session ID: 8-character prefix (e.g., "fb3cff44") or full UUID.
    /// Ambiguous prefixes matching multiple sessions will return an error with matched IDs.
    pub session_id: String,

    /// Response size control:
    /// - 'summary': 5-10 KB (turn count, no payloads)
    /// - 'turns': 15-30 KB (tool usage per turn)
    /// - 'steps': 50-100 KB (full structure, truncated payloads to 500 chars)
    /// - 'full': Unbounded (complete session, use with caution)
    ///
    /// Default: 'summary'
    #[serde(default)]
    pub detail_level: Option<DetailLevel>,

    /// Include <thinking> blocks in turn summaries.
    /// WARNING: Only valid when detail_level='turns'. Ignored for other levels.
    /// Adds ~5-10 KB per turn with reasoning content.
    /// Default: false
    #[serde(default)]
    pub include_reasoning: Option<bool>,
}

impl GetSessionDetailsArgs {
    pub fn detail_level(&self) -> DetailLevel {
        self.detail_level.unwrap_or_default()
    }

    pub fn include_reasoning(&self) -> bool {
        self.include_reasoning.unwrap_or(false)
    }
}

/// Run diagnostic analysis on a session to identify failures, loops, and issues
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeSessionArgs {
    /// Session ID to analyze
    pub session_id: String,
    /// Include failure analysis (default: true)
    #[serde(default)]
    pub include_failures: Option<bool>,
    /// Include loop detection (default: false)
    #[serde(default)]
    pub include_loops: Option<bool>,
}

/// Search for patterns in event payloads (returns previews only, ~300 char snippets)
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchEventPreviewsArgs {
    /// Search query (substring match in event JSON payloads)
    pub query: String,

    /// Maximum results per page (default: 10, max: 50)
    #[serde(default)]
    pub limit: Option<usize>,

    /// Pagination cursor from previous response's next_cursor field. Omit for first page.
    #[serde(default)]
    pub cursor: Option<String>,

    /// Filter by provider
    pub provider: Option<Provider>,

    /// Filter by event type (e.g., ToolCall, ToolResult, Message)
    pub event_type: Option<EventType>,

    /// Search within specific session only (optional)
    pub session_id: Option<String>,
}

/// Retrieve full event payload by session and index
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetEventDetailsArgs {
    /// Session ID: 8-character prefix or full UUID
    pub session_id: String,

    /// Zero-based event index within session
    pub event_index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::schema_for;

    #[test]
    fn test_list_sessions_schema() {
        let schema = schema_for!(ListSessionsArgs);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        println!("\n=== ListSessionsArgs Schema ===\n{}\n", json);

        // Verify required fields exist
        let schema_value = serde_json::to_value(&schema).unwrap();
        let properties = schema_value["properties"].as_object().unwrap();
        assert!(
            properties.contains_key("cursor"),
            "cursor field should exist in schema"
        );
        assert!(
            properties.contains_key("limit"),
            "limit field should exist in schema"
        );
        assert!(
            properties.contains_key("project_root"),
            "project_root field should exist in schema"
        );
        assert!(
            properties.contains_key("project_hash"),
            "project_hash field should exist in schema"
        );

        // Verify cursor description
        let cursor_desc = properties["cursor"]["description"].as_str().unwrap();
        assert!(
            cursor_desc.contains("cursor"),
            "cursor description should mention cursor"
        );

        // Verify project_root description
        let project_root_desc = properties["project_root"]["description"].as_str().unwrap();
        assert!(
            project_root_desc.contains("project root"),
            "project_root description should mention project root"
        );
        assert!(
            project_root_desc.contains("working directory") || project_root_desc.contains("cwd"),
            "project_root description should mention working directory or cwd"
        );
    }

    #[test]
    fn test_get_session_details_schema() {
        let schema = schema_for!(GetSessionDetailsArgs);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        println!("\n=== GetSessionDetailsArgs Schema ===\n{}\n", json);

        let schema_value = serde_json::to_value(&schema).unwrap();
        let properties = schema_value["properties"].as_object().unwrap();
        assert!(
            properties.contains_key("detail_level"),
            "detail_level should exist"
        );

        // Verify detail_level has enum
        let detail_level = &properties["detail_level"];
        assert!(
            detail_level.get("anyOf").is_some() || detail_level.get("enum").is_some(),
            "detail_level should have enum values"
        );

        // Verify required fields
        let required = schema_value["required"].as_array().unwrap();
        assert!(
            required.contains(&serde_json::Value::String("session_id".to_string())),
            "session_id should be in required array"
        );
        assert!(
            !required.contains(&serde_json::Value::String("detail_level".to_string())),
            "detail_level should NOT be in required array (it's optional)"
        );
        assert!(
            !required.contains(&serde_json::Value::String("include_reasoning".to_string())),
            "include_reasoning should NOT be in required array (it's optional)"
        );
    }
}
