use agtrace_types::{AgentEvent, ContextWindowUsage, EventPayload};
use serde_json::Value;

/// Pure data extracted from an AgentEvent to update runtime session state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StateUpdates {
    pub model: Option<String>,
    pub context_window_limit: Option<u64>,
    pub usage: Option<ContextWindowUsage>,
    pub reasoning_tokens: Option<i32>,
    pub is_error: bool,
    pub is_new_turn: bool,
}

/// Extract state updates from a single event without performing I/O or side effects.
pub fn extract_state_updates(event: &AgentEvent) -> StateUpdates {
    let mut updates = StateUpdates::default();

    match &event.payload {
        EventPayload::User(_) => {
            updates.is_new_turn = true;
        }
        EventPayload::TokenUsage(usage) => {
            // Convert normalized TokenUsagePayload to ContextWindowUsage
            // TODO: Verify if input.total() includes cache tokens (potential double counting)
            // The new TokenUsagePayload separates cached/uncached, but we use total() for compatibility
            // with the original logic. If so, fresh_input should be: usage.input.uncached only
            updates.usage = Some(ContextWindowUsage::from_raw(
                usage.input.total() as i32,  // input_tokens equivalent
                0,                            // cache_creation - not separately tracked
                usage.input.cached as i32,    // cache_read_input_tokens equivalent
                usage.output.total() as i32,  // output_tokens equivalent
            ));
            updates.reasoning_tokens = Some(usage.output.reasoning as i32);
        }
        EventPayload::ToolResult(result) => {
            if result.is_error {
                updates.is_error = true;
            } else {
                // Explicitly mark success so consumers can reset counters if needed.
                updates.is_error = false;
            }
        }
        _ => {}
    }

    if let Some(metadata) = &event.metadata {
        if updates.model.is_none() {
            updates.model = extract_model(metadata);
        }

        if updates.context_window_limit.is_none() {
            updates.context_window_limit = extract_context_window_limit(metadata);
        }
    }

    updates
}

fn extract_model(metadata: &Value) -> Option<String> {
    metadata
        .get("message")
        .and_then(|m| m.get("model"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            metadata
                .get("model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
}

fn extract_context_window_limit(metadata: &Value) -> Option<u64> {
    metadata
        .get("info")
        .and_then(|info| info.get("model_context_window"))
        .and_then(|v| v.as_u64())
        .or_else(|| {
            metadata
                .get("payload")
                .and_then(|payload| payload.get("info"))
                .and_then(|info| info.get("model_context_window"))
                .and_then(|v| v.as_u64())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_types::{
        TokenInput, TokenOutput, TokenUsagePayload, ToolResultPayload, UserPayload,
    };
    use chrono::Utc;
    use std::str::FromStr;
    use uuid::Uuid;

    fn base_event(payload: EventPayload) -> AgentEvent {
        AgentEvent {
            id: Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap(),
            session_id: Uuid::from_str("00000000-0000-0000-0000-000000000002").unwrap(),
            parent_id: None,
            timestamp: Utc::now(),
            stream_id: agtrace_types::StreamId::Main,
            payload,
            metadata: None,
        }
    }

    #[test]
    fn extracts_user_turn_flag() {
        let event = base_event(EventPayload::User(UserPayload {
            text: "hi".to_string(),
        }));

        let updates = extract_state_updates(&event);
        assert!(updates.is_new_turn);
        assert!(!updates.is_error);
    }

    #[test]
    fn extracts_token_usage_and_reasoning() {
        let mut event = base_event(EventPayload::TokenUsage(TokenUsagePayload::new(
            TokenInput::new(20, 100),   // cached=20, uncached=100
            TokenOutput::new(43, 7, 0), // generated=43, reasoning=7, tool=0
        )));

        let mut meta = serde_json::Map::new();
        meta.insert(
            "model".to_string(),
            serde_json::Value::String("claude-3-5-sonnet-20241022".to_string()),
        );
        meta.insert(
            "info".to_string(),
            serde_json::json!({ "model_context_window": 200000 }),
        );
        event.metadata = Some(Value::Object(meta));

        let updates = extract_state_updates(&event);

        let usage = updates.usage.expect("usage should be set");
        assert_eq!(usage.fresh_input.0, 120); // input.total() = cached + uncached = 20 + 100
        assert_eq!(usage.cache_read.0, 20); // cached input
        assert_eq!(usage.output.0, 50); // generated + reasoning + tool = 43 + 7 + 0
        assert_eq!(usage.total_tokens(), crate::TokenCount::new(190)); // 120 + 20 + 50 (note: fresh_input includes cache, so double counted)

        assert_eq!(updates.reasoning_tokens, Some(7));
        assert_eq!(
            updates.model,
            Some("claude-3-5-sonnet-20241022".to_string())
        );
        assert_eq!(updates.context_window_limit, Some(200_000));
    }

    #[test]
    fn extracts_context_window_limit_from_payload_info() {
        let mut event = base_event(EventPayload::TokenUsage(TokenUsagePayload::new(
            TokenInput::new(0, 10),    // cached=0, uncached=10
            TokenOutput::new(5, 0, 0), // generated=5, reasoning=0, tool=0
        )));

        event.metadata = Some(serde_json::json!({
            "payload": {
                "info": { "model_context_window": 123_000 }
            }
        }));

        let updates = extract_state_updates(&event);
        assert_eq!(updates.context_window_limit, Some(123_000));
    }

    #[test]
    fn extracts_tool_result_error_flag() {
        let event = base_event(EventPayload::ToolResult(ToolResultPayload {
            tool_call_id: Uuid::from_str("00000000-0000-0000-0000-000000000003").unwrap(),
            output: "err".to_string(),
            is_error: true,
        }));

        let updates = extract_state_updates(&event);
        assert!(updates.is_error);
    }

    #[test]
    fn applies_updates_to_session_state_without_io() {
        #[derive(Default)]
        struct SessionState {
            model: Option<String>,
            context_window_limit: Option<u64>,
            usage: ContextWindowUsage,
            reasoning_tokens: i32,
            turn_count: usize,
            error_count: u32,
        }

        impl SessionState {
            fn apply(&mut self, updates: StateUpdates, is_error_event: bool) {
                if updates.is_new_turn {
                    self.turn_count += 1;
                    self.error_count = 0;
                }
                if is_error_event && updates.is_error {
                    self.error_count += 1;
                }
                if let Some(m) = updates.model {
                    self.model.get_or_insert(m);
                }
                if let Some(limit) = updates.context_window_limit {
                    self.context_window_limit.get_or_insert(limit);
                }
                if let Some(u) = updates.usage {
                    self.usage = u;
                }
                if let Some(rt) = updates.reasoning_tokens {
                    self.reasoning_tokens = rt;
                }
            }
        }

        let user = base_event(EventPayload::User(UserPayload { text: "hi".into() }));
        let mut usage_event = base_event(EventPayload::TokenUsage(TokenUsagePayload::new(
            TokenInput::new(5, 120),    // cached=5, uncached=120
            TokenOutput::new(27, 3, 0), // generated=27, reasoning=3, tool=0
        )));
        let mut meta = serde_json::Map::new();
        meta.insert("model".into(), serde_json::Value::String("claude-3".into()));
        meta.insert(
            "info".into(),
            serde_json::json!({"model_context_window": 100000}),
        );
        usage_event.metadata = Some(Value::Object(meta));

        let tool_err = base_event(EventPayload::ToolResult(ToolResultPayload {
            tool_call_id: Uuid::from_str("00000000-0000-0000-0000-000000000009").unwrap(),
            output: "boom".into(),
            is_error: true,
        }));

        let mut state = SessionState::default();

        state.apply(extract_state_updates(&user), false);
        state.apply(extract_state_updates(&usage_event), false);
        state.apply(extract_state_updates(&tool_err), true);

        assert_eq!(state.turn_count, 1);
        assert_eq!(state.error_count, 1);
        assert_eq!(state.model.as_deref(), Some("claude-3"));
        assert_eq!(state.context_window_limit, Some(100_000));
        assert_eq!(state.usage.fresh_input.0, 125); // input.total() = cached + uncached = 5 + 120
        assert_eq!(state.usage.cache_read.0, 5);
        assert_eq!(state.usage.output.0, 30); // generated + reasoning + tool = 27 + 3 + 0
        assert_eq!(state.reasoning_tokens, 3);
    }
}
