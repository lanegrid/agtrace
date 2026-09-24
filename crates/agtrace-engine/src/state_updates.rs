use agtrace_types::{AgentEvent, ContextWindowHintPayload, ContextWindowUsage, EventPayload};

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
///
/// Model and context-window evidence come from typed payloads
/// (`TokenUsage.model`, `ModelChange`, `ContextWindowHint::Explicit`).
pub fn extract_state_updates(event: &AgentEvent) -> StateUpdates {
    let mut updates = StateUpdates::default();

    match &event.payload {
        EventPayload::User(_) | EventPayload::SlashCommand(_) => {
            updates.is_new_turn = true;
        }
        EventPayload::TokenUsage(usage) => {
            // fresh_input = uncached only (not total) to avoid double-counting cache reads.
            updates.usage = Some(ContextWindowUsage::from_token_usage(usage));
            updates.reasoning_tokens = Some(usage.output.reasoning as i32);
            updates.model = usage.model.clone();
        }
        EventPayload::ModelChange(change) => {
            updates.model = Some(change.to.clone());
        }
        EventPayload::ContextWindowHint(ContextWindowHintPayload::Explicit { tokens, model }) => {
            updates.context_window_limit = Some(*tokens);
            updates.model = model.clone();
        }
        EventPayload::ToolResult(result) => {
            // Explicitly mark success so consumers can reset counters if needed.
            updates.is_error = result.is_error;
        }
        _ => {}
    }

    updates
}

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_types::{
        AgentId, EventOrigin, TokenInput, TokenOutput, TokenUsagePayload, ToolResultPayload,
        UserPayload,
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
            agent: AgentId::claude_session("s"),
            origin: EventOrigin::default(),
            payload,
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
    fn extracts_token_usage_model_and_reasoning() {
        let event = base_event(EventPayload::TokenUsage(
            TokenUsagePayload::new(
                TokenInput::new(100, 20, 0), // uncached=100, cache_read=20
                TokenOutput::new(43, 7, 0),  // generated=43, reasoning=7, tool=0
            )
            .with_model(Some("claude-3-5-sonnet-20241022".to_string())),
        ));

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
        assert_eq!(updates.context_window_limit, None);
    }

    #[test]
    fn extracts_cache_write_as_cache_creation() {
        let event = base_event(EventPayload::TokenUsage(TokenUsagePayload::new(
            TokenInput::new(10, 100, 30),
            TokenOutput::new(5, 0, 0),
        )));
        let usage = extract_state_updates(&event).usage.unwrap();
        assert_eq!(usage.cache_creation.0, 30);
        assert_eq!(usage.input_tokens(), 140);
    }

    #[test]
    fn extracts_context_window_limit_from_hint() {
        let event = base_event(EventPayload::ContextWindowHint(
            ContextWindowHintPayload::Explicit {
                tokens: 123_000,
                model: Some("gpt-5.6-sol".to_string()),
            },
        ));

        let updates = extract_state_updates(&event);
        assert_eq!(updates.context_window_limit, Some(123_000));
        assert_eq!(updates.model.as_deref(), Some("gpt-5.6-sol"));
    }

    #[test]
    fn extracts_model_from_model_change() {
        let event = base_event(EventPayload::ModelChange(
            agtrace_types::ModelChangePayload {
                from: None,
                to: "claude-opus-5-5".to_string(),
                source: agtrace_types::ModelChangeSource::AssistantMessage,
            },
        ));
        assert_eq!(
            extract_state_updates(&event).model.as_deref(),
            Some("claude-opus-5-5")
        );
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
            TokenInput::new(100, 20, 0), // uncached=100, cache_read=20
            TokenOutput::new(50, 0, 0),  // generated=50, reasoning=0, tool=0
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
            TokenInput::new(200, 30, 0), // uncached=200, cache_read=30
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
        let hint_event = base_event(EventPayload::ContextWindowHint(
            ContextWindowHintPayload::Explicit {
                tokens: 100_000,
                model: None,
            },
        ));
        let usage_event = base_event(EventPayload::TokenUsage(
            TokenUsagePayload::new(
                TokenInput::new(120, 5, 0), // uncached=120, cache_read=5
                TokenOutput::new(27, 3, 0), // generated=27, reasoning=3, tool=0
            )
            .with_model(Some("claude-3".to_string())),
        ));

        let tool_err = base_event(EventPayload::ToolResult(ToolResultPayload {
            tool_call_id: Uuid::from_str("00000000-0000-0000-0000-000000000009").unwrap(),
            output: "boom".into(),
            is_error: true,
            agent_id: None,
        }));

        let mut state = SessionState::default();

        state.apply(extract_state_updates(&user), false);
        state.apply(extract_state_updates(&hint_event), false);
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
