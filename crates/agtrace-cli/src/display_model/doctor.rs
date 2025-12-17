use agtrace_types::{AgentEvent, EventPayload};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DoctorCheckDisplay {
    pub file_path: String,
    pub provider_name: String,
    pub result: CheckResult,
}

#[derive(Debug, Clone)]
pub enum CheckResult {
    Valid {
        trace_id: String,
        timestamp: DateTime<Utc>,
        event_count: usize,
        event_breakdown: HashMap<String, usize>,
    },
    Invalid {
        error_message: String,
        suggestion: Option<String>,
    },
}

impl DoctorCheckDisplay {
    pub fn from_events(file_path: String, provider_name: String, events: Vec<AgentEvent>) -> Self {
        let first_event = events.first();
        let trace_id = first_event
            .map(|e| e.trace_id.to_string())
            .unwrap_or_default();
        let timestamp = first_event.map(|e| e.timestamp).unwrap_or_else(Utc::now);

        let mut event_breakdown = HashMap::new();
        for event in &events {
            let payload_type = match &event.payload {
                EventPayload::User(_) => "User",
                EventPayload::Message(_) => "Message",
                EventPayload::ToolCall(_) => "ToolCall",
                EventPayload::ToolResult(_) => "ToolResult",
                EventPayload::Reasoning(_) => "Reasoning",
                EventPayload::TokenUsage(_) => "TokenUsage",
                EventPayload::Notification(_) => "Notification",
            };
            *event_breakdown.entry(payload_type.to_string()).or_insert(0) += 1;
        }

        Self {
            file_path,
            provider_name,
            result: CheckResult::Valid {
                trace_id,
                timestamp,
                event_count: events.len(),
                event_breakdown,
            },
        }
    }

    pub fn from_error(file_path: String, provider_name: String, error: anyhow::Error) -> Self {
        let error_message = format!("{:#}", error);
        let suggestion = generate_suggestion(&error_message, &file_path);

        Self {
            file_path,
            provider_name,
            result: CheckResult::Invalid {
                error_message,
                suggestion,
            },
        }
    }
}

fn generate_suggestion(error_msg: &str, file_path: &str) -> Option<String> {
    if error_msg.contains("missing field") {
        Some(
            "This field may have been added in a newer version of the provider.\n\
             Check if the schema definition needs to make this field optional."
                .to_string(),
        )
    } else if error_msg.contains("invalid type") {
        Some(format!(
            "The field type in the schema may not match the actual data format.\n\
             Use 'agtrace inspect {}' to examine the actual structure.\n\
             Use 'agtrace schema <provider>' to see the expected format.",
            file_path
        ))
    } else if error_msg.contains("expected") {
        Some(
            "The data format may have changed between provider versions.\n\
             Consider using an enum or untagged union to support multiple formats."
                .to_string(),
        )
    } else {
        None
    }
}
