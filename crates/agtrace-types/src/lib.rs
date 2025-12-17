pub mod models;
mod util;

use serde::{Deserialize, Serialize};

pub use models::*;
pub use util::*;

/// Git repository context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_dirty: Option<bool>,
}

/// Execution environment context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<GitContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
}

/// Agent control policy and constraints
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_access: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<String>,
}

/// Source of the agent log (provider-agnostic identifier)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Source(String);

impl Source {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// Tool execution status (used in Span API)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Success,
    Error,
    InProgress,
    Unknown,
}

/// Session summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub source: Source,
    pub project_hash: String,
    pub start_ts: String,
    pub end_ts: String,
    pub event_count: usize,
    pub user_message_count: usize,
    pub tokens_input_total: u64,
    pub tokens_output_total: u64,
}
