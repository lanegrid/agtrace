use crate::model::Role;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Activity {
    Message {
        role: Role,
        text: String,
        timestamp: String,
        duration_ms: Option<u64>,
        stats: ActivityStats,
    },
    Execution {
        timestamp: String,
        duration_ms: u64,
        status: ActivityStatus,
        tools: Vec<ToolSummary>,
        stats: ActivityStats,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSummary {
    pub name: String,
    pub input_summary: String,
    pub count: usize,
    pub is_error: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityStats {
    pub total_tokens: Option<u64>,
    pub event_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityStatus {
    Success,
    Failure,
    LongRunning,
}

impl Activity {
    pub fn timestamp(&self) -> &str {
        match self {
            Activity::Message { timestamp, .. } => timestamp,
            Activity::Execution { timestamp, .. } => timestamp,
        }
    }

    pub fn duration_ms(&self) -> Option<u64> {
        match self {
            Activity::Message { duration_ms, .. } => *duration_ms,
            Activity::Execution { duration_ms, .. } => Some(*duration_ms),
        }
    }

    pub fn stats(&self) -> &ActivityStats {
        match self {
            Activity::Message { stats, .. } => stats,
            Activity::Execution { stats, .. } => stats,
        }
    }
}
