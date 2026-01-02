use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::DetailLevel;

/// List recent AI agent sessions with cursor-based pagination
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListSessionsArgs {
    /// Maximum number of sessions to return (default: 10, max: 50)
    #[serde(default)]
    pub limit: Option<usize>,
    /// Pagination cursor from previous response's next_cursor field. Omit for first page.
    #[serde(default)]
    pub cursor: Option<String>,
    /// Filter by provider (claude_code, codex, gemini)
    pub provider: Option<String>,
    /// Filter by project hash
    pub project_hash: Option<String>,
    /// Show sessions after this timestamp (ISO 8601)
    pub since: Option<String>,
    /// Show sessions before this timestamp (ISO 8601)
    pub until: Option<String>,
}

/// Get session details with configurable verbosity
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetSessionDetailsArgs {
    /// Session ID (short or full UUID)
    pub session_id: String,
    /// Detail level: 'summary' (5-10KB), 'turns' (15-30KB), 'steps' (50-100KB), or 'full' (complete data)
    #[serde(default)]
    pub detail_level: Option<DetailLevel>,
    /// Include reasoning/thinking content in summaries (only applies to 'turns' level)
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

/// Search for patterns in event payloads across recent sessions with cursor-based pagination
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchEventsArgs {
    /// Search pattern (substring match)
    pub pattern: String,
    /// Maximum number of matches (default: 5, max: 20)
    #[serde(default)]
    pub limit: Option<usize>,
    /// Pagination cursor from previous response's next_cursor field. Omit for first page.
    #[serde(default)]
    pub cursor: Option<String>,
    /// Filter by provider
    pub provider: Option<String>,
    /// Filter by event type
    pub event_type: Option<String>,
    /// Include full event payload (default: false). WARNING: Can produce large responses.
    #[serde(default)]
    pub include_full_payload: Option<bool>,
}
