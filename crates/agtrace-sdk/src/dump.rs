use agtrace_engine::AgentSession;
use agtrace_types::domain::{
    ContextWindowUsage, MessageBlock, ReasoningBlock, ToolExecution, UserMessage,
};
use serde::{Deserialize, Serialize};

/// A flattened event record with sequence number and hierarchy context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    /// Sequential event number within the session (0-indexed).
    pub seq: usize,
    /// Event timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Event type (User, Message, ToolCall, ToolResult, TokenUsage, etc.).
    pub r#type: String,
    /// Normalized event content.
    pub content: serde_json::Value,
    /// Index of the turn this event belongs to.
    pub turn_idx: usize,
    /// Index of the step this event belongs to (None for turn-level events like User).
    pub step_idx: Option<usize>,
}

/// A flattened event record with raw provider metadata for normalization verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEventRecord {
    /// Sequential event number within the session (0-indexed).
    pub seq: usize,
    /// Event timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Event type (User, Message, ToolCall, ToolResult, TokenUsage, etc.).
    pub r#type: String,
    /// Normalized event content.
    pub normalized: serde_json::Value,
    /// Provider information.
    pub provider: ProviderInfo,
    /// Index of the turn this event belongs to.
    pub turn_idx: usize,
    /// Index of the step this event belongs to (None for turn-level events).
    pub step_idx: Option<usize>,
}

/// Provider-specific information for debugging normalization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Provider name (claude_code, codex, gemini).
    pub name: String,
    /// Raw provider-specific payload.
    pub raw_payload: serde_json::Value,
}

/// Convert an AgentSession into a flat stream of event records.
///
/// This flattens the hierarchical structure (session → turns → steps → events)
/// into a chronological sequence suitable for debugging and analysis.
pub fn session_to_event_stream(
    session: &AgentSession,
    include_raw: bool,
) -> crate::Result<Vec<serde_json::Value>> {
    let mut events = Vec::new();
    let mut seq = 0;

    for (turn_idx, turn) in session.turns.iter().enumerate() {
        // Add User event (turn-level)
        let user_event = create_user_event(seq, turn_idx, &turn.user, turn.timestamp);
        events.push(user_event);
        seq += 1;

        // Add step-level events
        for (step_idx, step) in turn.steps.iter().enumerate() {
            // Add Reasoning event (if present)
            if let Some(ref reasoning) = step.reasoning {
                let reasoning_event =
                    create_reasoning_event(seq, turn_idx, step_idx, reasoning, step.timestamp);
                events.push(reasoning_event);
                seq += 1;
            }

            // Add Message event (if present)
            if let Some(ref message) = step.message {
                let message_event =
                    create_message_event(seq, turn_idx, step_idx, message, step.timestamp);
                events.push(message_event);
                seq += 1;
            }

            // Add ToolCall and ToolResult events
            for tool_exec in &step.tools {
                let tool_call_event = create_tool_call_event(
                    seq,
                    turn_idx,
                    step_idx,
                    tool_exec,
                    tool_exec.call.timestamp,
                );
                events.push(tool_call_event);
                seq += 1;

                // Add ToolResult event (if present)
                if tool_exec.result.is_some() {
                    let tool_result_event = create_tool_result_event(
                        seq,
                        turn_idx,
                        step_idx,
                        tool_exec,
                        tool_exec.result.as_ref().unwrap().timestamp,
                    );
                    events.push(tool_result_event);
                    seq += 1;
                }
            }

            // Add TokenUsage event (if present)
            if let Some(ref usage) = step.usage {
                let token_usage_event =
                    create_token_usage_event(seq, turn_idx, step_idx, usage, step.timestamp);
                events.push(token_usage_event);
                seq += 1;
            }
        }
    }

    // Convert to JSON values based on raw flag
    let output: Vec<serde_json::Value> = if include_raw {
        // Note: Raw mode requires access to original AgentEvent metadata
        // which is not available in the assembled AgentSession.
        // For now, return normalized events with a placeholder for raw.
        // TODO: Preserve original event metadata in AgentSession or fetch from source
        events
    } else {
        events
    };

    Ok(output)
}

fn create_user_event(
    seq: usize,
    turn_idx: usize,
    user: &UserMessage,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> serde_json::Value {
    serde_json::json!({
        "seq": seq,
        "timestamp": timestamp.to_rfc3339(),
        "type": "User",
        "content": {
            "text": user.content.text
        },
        "turn_idx": turn_idx,
        "step_idx": null
    })
}

fn create_message_event(
    seq: usize,
    turn_idx: usize,
    step_idx: usize,
    message: &MessageBlock,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> serde_json::Value {
    serde_json::json!({
        "seq": seq,
        "timestamp": timestamp.to_rfc3339(),
        "type": "Message",
        "content": {
            "text": message.content.text
        },
        "turn_idx": turn_idx,
        "step_idx": step_idx
    })
}

fn create_reasoning_event(
    seq: usize,
    turn_idx: usize,
    step_idx: usize,
    reasoning: &ReasoningBlock,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> serde_json::Value {
    serde_json::json!({
        "seq": seq,
        "timestamp": timestamp.to_rfc3339(),
        "type": "Reasoning",
        "content": {
            "text": reasoning.content.text
        },
        "turn_idx": turn_idx,
        "step_idx": step_idx
    })
}

fn create_tool_call_event(
    seq: usize,
    turn_idx: usize,
    step_idx: usize,
    tool_exec: &ToolExecution,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> serde_json::Value {
    // Serialize the entire tool call payload to get arguments as JSON
    let payload_json =
        serde_json::to_value(&tool_exec.call.content).unwrap_or(serde_json::json!({}));

    serde_json::json!({
        "seq": seq,
        "timestamp": timestamp.to_rfc3339(),
        "type": "ToolCall",
        "content": {
            "name": tool_exec.call.content.name(),
            "payload": payload_json,
        },
        "turn_idx": turn_idx,
        "step_idx": step_idx
    })
}

fn create_tool_result_event(
    seq: usize,
    turn_idx: usize,
    step_idx: usize,
    tool_exec: &ToolExecution,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> serde_json::Value {
    let result = tool_exec.result.as_ref().unwrap();
    serde_json::json!({
        "seq": seq,
        "timestamp": timestamp.to_rfc3339(),
        "type": "ToolResult",
        "content": {
            "output": result.content.output,
            "is_error": result.content.is_error
        },
        "turn_idx": turn_idx,
        "step_idx": step_idx
    })
}

fn create_token_usage_event(
    seq: usize,
    turn_idx: usize,
    step_idx: usize,
    usage: &ContextWindowUsage,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> serde_json::Value {
    serde_json::json!({
        "seq": seq,
        "timestamp": timestamp.to_rfc3339(),
        "type": "TokenUsage",
        "content": {
            "fresh_input": usage.fresh_input.0,
            "cache_read": usage.cache_read.0,
            "output": usage.output.0,
            "total": usage.total_tokens().as_u64()
        },
        "turn_idx": turn_idx,
        "step_idx": step_idx
    })
}
