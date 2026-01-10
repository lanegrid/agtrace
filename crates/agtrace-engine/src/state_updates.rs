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
        EventPayload::User(_) | EventPayload::SlashCommand(_) => {
            updates.is_new_turn = true;
        }
        EventPayload::TokenUsage(usage) => {
            // Convert normalized TokenUsagePayload to ContextWindowUsage
            // The new TokenUsagePayload separates input into cached/uncached.
            // To avoid double-counting, fresh_input = uncached only (not total).
            updates.usage = Some(ContextWindowUsage::from_raw(
                usage.input.uncached as i32, // fresh input tokens (not from cache)
                0,                           // cache_creation - not separately tracked
                usage.input.cached as i32,   // cache_read tokens (still consume context)
                usage.output.total() as i32, // total output tokens
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
        assert_eq!(usage.fresh_input.0, 100); // uncached only (not total)
        assert_eq!(usage.cache_read.0, 20); // cached input
        assert_eq!(usage.output.0, 50); // generated + reasoning + tool = 43 + 7 + 0
        assert_eq!(usage.total_tokens(), crate::TokenCount::new(170)); // 100 + 20 + 50

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
            agent_id: None,
        }));

        let updates = extract_state_updates(&event);
        assert!(updates.is_error);
    }

    #[test]
    fn token_usage_conversion_avoids_double_counting_cached_tokens() {
        // Bug reproduction test: cached tokens should NOT be counted twice
        //
        // Given a TokenUsagePayload with:
        //   input:  cached=20, uncached=100 (total input = 120)
        //   output: generated=50 (total output = 50)
        //
        // Expected ContextWindowUsage:
        //   fresh_input:    100 (uncached only)
        //   cache_read:      20 (cached tokens)
        //   output:          50
        //   total_tokens:   170 (100 + 20 + 50)
        //
        // Bug produces:
        //   fresh_input:    120 (input.total() = cached + uncached)
        //   cache_read:      20 (same)
        //   total_tokens:   190 (120 + 20 + 50) ❌ cached counted twice!

        let event = base_event(EventPayload::TokenUsage(TokenUsagePayload::new(
            TokenInput::new(20, 100),   // cached=20, uncached=100
            TokenOutput::new(50, 0, 0), // generated=50, reasoning=0, tool=0
        )));

        let updates = extract_state_updates(&event);
        let usage = updates.usage.expect("usage should be set");

        // CORRECT expectations (this test will FAIL until bug is fixed):
        assert_eq!(
            usage.fresh_input.0, 100,
            "fresh_input should be uncached tokens only (not total)"
        );
        assert_eq!(usage.cache_read.0, 20, "cache_read should be cached tokens");
        assert_eq!(usage.output.0, 50, "output should match");
        assert_eq!(
            usage.total_tokens(),
            crate::TokenCount::new(170),
            "total should be 100 (fresh) + 20 (cache) + 50 (output) = 170, not 190"
        );
    }

    #[test]
    fn token_usage_conversion_uses_uncached_for_fresh_input() {
        // Consistency test: The conversion logic should match merge_usage semantics
        // which correctly uses input.uncached for fresh_input (not input.total())
        //
        // This ensures extract_state_updates produces the same result as the
        // conversion done in session assembly (stats::merge_usage)

        let token_payload = TokenUsagePayload::new(
            TokenInput::new(30, 200),    // cached=30, uncached=200
            TokenOutput::new(80, 10, 5), // generated=80, reasoning=10, tool=5
        );

        let event = base_event(EventPayload::TokenUsage(token_payload));
        let updates = extract_state_updates(&event);
        let usage = updates.usage.expect("usage should be set");

        // Should use uncached only for fresh_input (matching merge_usage logic)
        assert_eq!(
            usage.fresh_input.0, 200,
            "fresh_input must be uncached tokens only (200), not total (230)"
        );
        assert_eq!(usage.cache_read.0, 30);
        assert_eq!(usage.output.0, 95); // 80 + 10 + 5
        assert_eq!(usage.total_tokens(), crate::TokenCount::new(325)); // 200 + 30 + 95
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
            agent_id: None,
        }));

        let mut state = SessionState::default();

        state.apply(extract_state_updates(&user), false);
        state.apply(extract_state_updates(&usage_event), false);
        state.apply(extract_state_updates(&tool_err), true);

        assert_eq!(state.turn_count, 1);
        assert_eq!(state.error_count, 1);
        assert_eq!(state.model.as_deref(), Some("claude-3"));
        assert_eq!(state.context_window_limit, Some(100_000));
        assert_eq!(state.usage.fresh_input.0, 120); // uncached only (not total)
        assert_eq!(state.usage.cache_read.0, 5);
        assert_eq!(state.usage.output.0, 30); // generated + reasoning + tool = 27 + 3 + 0
        assert_eq!(state.reasoning_tokens, 3);
    }
}
