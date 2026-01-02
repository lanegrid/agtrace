use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::super::common::Provider;

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

#[derive(Debug, Serialize)]
pub struct ListSessionsViewModel {
    pub sessions: Vec<agtrace_sdk::SessionSummary>,
    pub total_in_page: usize,
    pub next_cursor: Option<String>,
}

impl ListSessionsViewModel {
    pub fn new(sessions: Vec<agtrace_sdk::SessionSummary>, next_cursor: Option<String>) -> Self {
        let total_in_page = sessions.len();
        Self {
            sessions,
            total_in_page,
            next_cursor,
        }
    }
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
