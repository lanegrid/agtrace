// MCP Tool Request Types
//
// Design Notes:
// - All schemas auto-generated from Rust types via schemars (single source of truth)
// - DetailLevel: Progressive disclosure for get_session_details (summary → turns → steps → full)
// - SearchEventPreviewsArgs: Separated from deprecated SearchEventsArgs for clarity

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::{EventType, Provider};

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

// ============================================================================
// Specialized Session Tools
// ============================================================================

/// Get lightweight session overview (≤5 KB, always single-page)
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetSessionSummaryArgs {
    /// Session ID obtained from list_sessions response (use the 'id' field).
    /// Accepts 8-character prefix (e.g., "fb3cff44") or full UUID.
    /// REQUIRED: Cannot be empty.
    pub session_id: String,
}

/// Get turn-level summaries with pagination (10-30 KB per page)
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetSessionTurnsArgs {
    /// Session ID obtained from list_sessions response (use the 'id' field).
    /// Accepts 8-character prefix (e.g., "fb3cff44") or full UUID.
    /// REQUIRED: Cannot be empty.
    pub session_id: String,

    /// Pagination cursor from previous response's next_cursor field. Omit for first page.
    #[serde(default)]
    pub cursor: Option<String>,

    /// Maximum number of turns to return per page (default: 10, max: 50)
    #[serde(default)]
    pub limit: Option<usize>,
}

impl GetSessionTurnsArgs {
    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(10).min(50)
    }
}

/// Get detailed steps for a specific turn (20-50 KB)
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetTurnStepsArgs {
    /// Session ID obtained from list_sessions or get_session_turns response.
    /// Accepts 8-character prefix (e.g., "fb3cff44") or full UUID.
    /// REQUIRED: Cannot be empty.
    pub session_id: String,

    /// Zero-based turn index (obtained from get_session_turns response).
    /// REQUIRED: Must be valid (0 to turn_count - 1).
    pub turn_index: usize,

    /// Pagination cursor for turns with many steps. Omit for first page.
    #[serde(default)]
    pub cursor: Option<String>,

    /// Maximum number of steps to return (default: 20, max: 100)
    #[serde(default)]
    pub limit: Option<usize>,
}

impl GetTurnStepsArgs {
    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(20).min(100)
    }
}

/// Get complete session data with full payloads (50-100 KB per chunk, requires pagination)
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetSessionFullArgs {
    /// Session ID obtained from list_sessions response (use the 'id' field).
    /// Accepts 8-character prefix (e.g., "fb3cff44") or full UUID.
    /// REQUIRED: Cannot be empty.
    pub session_id: String,

    /// Pagination cursor. Use null/"start" for first call, then next_cursor from responses.
    /// REQUIRED for safety: forces explicit pagination awareness.
    #[serde(default)]
    pub cursor: Option<String>,

    /// Maximum number of turns per chunk (default: 5, max: 10).
    /// Kept small to ensure chunks stay under 100 KB.
    #[serde(default)]
    pub limit: Option<usize>,
}

impl GetSessionFullArgs {
    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(5).min(10)
    }

    /// Check if this is the initial request (no cursor or "start")
    pub fn is_initial(&self) -> bool {
        self.cursor.is_none() || self.cursor.as_deref() == Some("start")
    }
}

/// Run diagnostic analysis on a session to identify failures, loops, and issues
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeSessionArgs {
    /// Session ID obtained from list_sessions response (use the 'id' field).
    /// Accepts 8-character prefix (e.g., "fb3cff44") or full UUID.
    /// REQUIRED: Cannot be empty.
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
    /// Session ID obtained from search_event_previews response (use the 'session_id' field).
    /// Accepts 8-character prefix (e.g., "fb3cff44") or full UUID.
    /// REQUIRED: Cannot be empty.
    pub session_id: String,

    /// Zero-based event index obtained from search_event_previews response (use the 'event_index' field).
    /// REQUIRED: Must specify a valid index (0 to session event count - 1).
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
}
