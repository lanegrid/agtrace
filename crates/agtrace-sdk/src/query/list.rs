//! List sessions query types.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Provider;
use crate::SessionSummary;

/// List recent AI agent sessions with cursor-based pagination.
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
    /// Include child sessions (subagents) in the list. By default, only top-level sessions are shown.
    #[serde(default)]
    pub include_children: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ListSessionsViewModel {
    pub sessions: Vec<SessionSummary>,
    pub total_in_page: usize,
    pub next_cursor: Option<String>,
}

impl ListSessionsViewModel {
    pub fn new(sessions: Vec<SessionSummary>, next_cursor: Option<String>) -> Self {
        let total_in_page = sessions.len();
        Self {
            sessions,
            total_in_page,
            next_cursor,
        }
    }
}
