use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct LabExportViewModel {
    pub exported_count: usize,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolCallSample {
    pub arguments: String,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolClassification {
    pub tool_name: String,
    pub origin: Option<String>,
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolStatsEntry {
    pub tool_name: String,
    pub count: usize,
    pub sample: Option<ToolCallSample>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderStats {
    pub provider_name: String,
    pub tools: Vec<ToolStatsEntry>,
    pub classifications: Vec<ToolClassification>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LabStatsViewModel {
    pub total_sessions: usize,
    pub providers: Vec<ProviderStats>,
}

// Lab grep ViewModels
use agtrace_sdk::types::StreamId;
use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct EventViewModel {
    pub id: String,
    pub session_id: String,
    pub parent_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub stream_id: StreamId,
    pub payload: EventPayloadViewModel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventPayloadViewModel {
    User {
        text: String,
    },
    Reasoning {
        text: String,
    },
    ToolCall {
        name: String,
        arguments: Value,
    },
    ToolResult {
        output: String,
        is_error: bool,
    },
    Message {
        text: String,
    },
    TokenUsage {
        input: i32,
        output: i32,
        total: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_creation: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_read: Option<i32>,
    },
    Notification {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        level: Option<String>,
    },
    SlashCommand {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<String>,
    },
    QueueOperation {
        operation: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<String>,
    },
    Summary {
        summary: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        leaf_uuid: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct LabGrepViewModel {
    pub pattern: String,
    pub matches: Vec<EventViewModel>,
    pub json_output: bool,
}
