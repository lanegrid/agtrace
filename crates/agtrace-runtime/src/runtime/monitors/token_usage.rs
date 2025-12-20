use crate::domain::{SessionState, TokenLimits};
use crate::runtime::reactor::{Reaction, Reactor, ReactorContext};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};

// NOTE: TokenUsageMonitor Design Rationale
//
// Why monitor token usage?
// - Context window limits can be reached silently (no API error until overflow)
// - Approaching limit degrades response quality (less room for output)
// - Early warning enables user to start fresh session before hitting limit
//
// Why two thresholds (warning + critical)?
// - 80% warning: User has time to wrap up current task
// - 95% critical: Imminent limit, urgent action needed
// - Progressive alerts match severity to urgency

pub struct TokenUsageMonitor {
    limits: TokenLimits,
    /// Warning threshold (percentage, e.g., 80.0)
    warning_threshold: f64,
    /// Critical threshold (percentage, e.g., 95.0)
    critical_threshold: f64,
    /// Last time we notified about warning
    last_warning: Option<DateTime<Utc>>,
    /// Last time we notified about critical
    last_critical: Option<DateTime<Utc>>,
    /// Cooldown period between notifications
    notify_cooldown: Duration,
}

impl TokenUsageMonitor {
    pub fn new(warning_threshold: f64, critical_threshold: f64) -> Self {
        Self {
            limits: TokenLimits::new(),
            warning_threshold,
            critical_threshold,
            last_warning: None,
            last_critical: None,
            notify_cooldown: Duration::seconds(300), // 5 minutes
        }
    }

    pub fn default_thresholds() -> Self {
        Self::new(80.0, 95.0)
    }

    fn check_threshold(&mut self, state: &SessionState) -> Option<Reaction> {
        let (input_pct, output_pct, total_pct) =
            self.limits.get_usage_percentage_from_state(state)?;

        let total_tokens = state.total_context_window_tokens() as u64;
        let model = state.model.as_ref()?;

        let now = Utc::now();

        // Check critical threshold (95%)
        if total_pct >= self.critical_threshold {
            let should_notify = match self.last_critical {
                None => true,
                Some(last) => (now - last) > self.notify_cooldown,
            };

            if should_notify {
                self.last_critical = Some(now);
                return Some(Reaction::Warn(format!(
                    "Token usage critical: {:.1}% ({}/{} tokens). Consider starting a new session.",
                    total_pct,
                    total_tokens,
                    self.limits.get_limit(model)?.total_limit
                )));
            }
        }
        // Check warning threshold (80%)
        else if total_pct >= self.warning_threshold {
            let should_notify = match self.last_warning {
                None => true,
                Some(last) => (now - last) > self.notify_cooldown,
            };

            if should_notify {
                self.last_warning = Some(now);
                return Some(Reaction::Warn(format!(
                    "Token usage warning: {:.1}% (in: {:.1}%, out: {:.1}%). {}/{} tokens used.",
                    total_pct,
                    input_pct,
                    output_pct,
                    total_tokens,
                    self.limits.get_limit(model)?.total_limit
                )));
            }
        } else {
            // Reset notification state if usage drops below warning threshold
            self.last_warning = None;
            self.last_critical = None;
        }

        None
    }
}

impl Reactor for TokenUsageMonitor {
    fn name(&self) -> &str {
        "TokenUsageMonitor"
    }

    fn handle(&mut self, ctx: ReactorContext) -> Result<Reaction> {
        // Only check on TokenUsage events
        if !matches!(
            ctx.event.payload,
            agtrace_types::EventPayload::TokenUsage(_)
        ) {
            return Ok(Reaction::Continue);
        }

        // Need model info to check limits
        if ctx.state.model.is_none() {
            return Ok(Reaction::Continue);
        }

        if let Some(reaction) = self.check_threshold(ctx.state) {
            return Ok(reaction);
        }

        Ok(Reaction::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SessionState;
    use agtrace_engine::ContextWindowUsage;
    use agtrace_types::{
        AgentEvent, EventPayload, StreamId, TokenUsageDetails, TokenUsagePayload, UserPayload,
    };
    use chrono::Utc;

    fn create_token_usage_event(input_tokens: i32, output_tokens: i32) -> AgentEvent {
        use std::str::FromStr;
        let id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap();
        let session_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000002").unwrap();

        AgentEvent {
            id,
            session_id,
            parent_id: None,
            timestamp: Utc::now(),
            stream_id: StreamId::Main,
            payload: EventPayload::TokenUsage(TokenUsagePayload {
                input_tokens,
                output_tokens,
                total_tokens: input_tokens + output_tokens,
                details: Some(TokenUsageDetails {
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                    reasoning_output_tokens: None,
                }),
            }),
            metadata: None,
        }
    }

    fn create_user_event() -> AgentEvent {
        use std::str::FromStr;
        let id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000003").unwrap();
        let session_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000004").unwrap();

        AgentEvent {
            id,
            session_id,
            parent_id: None,
            timestamp: Utc::now(),
            stream_id: StreamId::Main,
            payload: EventPayload::User(UserPayload {
                text: "test".to_string(),
            }),
            metadata: None,
        }
    }

    #[test]
    fn test_below_threshold() {
        let mut monitor = TokenUsageMonitor::default_thresholds();
        let event = create_token_usage_event(10_000, 1_000);

        let mut state = SessionState::new("test".to_string(), None, Utc::now());
        state.model = Some("claude-3-5-sonnet-20241022".to_string());
        state.current_usage = ContextWindowUsage::from_raw(10_000, 0, 0, 1_000);

        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = monitor.handle(ctx).unwrap();
        assert!(matches!(result, Reaction::Continue));
    }

    #[test]
    fn test_warning_threshold() {
        let mut monitor = TokenUsageMonitor::default_thresholds();
        let event = create_token_usage_event(160_000, 10_000);

        let mut state = SessionState::new("test".to_string(), None, Utc::now());
        state.model = Some("claude-3-5-sonnet-20241022".to_string());
        state.current_usage = ContextWindowUsage::from_raw(160_000, 0, 0, 10_000);

        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = monitor.handle(ctx).unwrap();
        match result {
            Reaction::Warn(msg) => {
                assert!(msg.contains("80"));
                assert!(msg.contains("warning"));
            }
            _ => panic!("Expected Warn reaction"),
        }
    }

    #[test]
    fn test_critical_threshold() {
        let mut monitor = TokenUsageMonitor::default_thresholds();
        let event = create_token_usage_event(190_000, 5_000);

        let mut state = SessionState::new("test".to_string(), None, Utc::now());
        state.model = Some("claude-3-5-sonnet-20241022".to_string());
        state.current_usage = ContextWindowUsage::from_raw(190_000, 0, 0, 5_000);

        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = monitor.handle(ctx).unwrap();
        match result {
            Reaction::Warn(reason) => {
                assert!(reason.contains("critical"));
                assert!(reason.contains("97.5"));
            }
            _ => panic!("Expected Warn reaction"),
        }
    }

    #[test]
    fn test_non_token_usage_event_ignored() {
        let mut monitor = TokenUsageMonitor::default_thresholds();
        let event = create_user_event();

        let state = SessionState::new("test".to_string(), None, Utc::now());
        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = monitor.handle(ctx).unwrap();
        assert!(matches!(result, Reaction::Continue));
    }

    #[test]
    fn test_no_model_info() {
        let mut monitor = TokenUsageMonitor::default_thresholds();
        let event = create_token_usage_event(100_000, 10_000);

        let mut state = SessionState::new("test".to_string(), None, Utc::now());
        state.model = None; // No model info
        state.current_usage = ContextWindowUsage::from_raw(100_000, 0, 0, 10_000);

        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = monitor.handle(ctx).unwrap();
        assert!(matches!(result, Reaction::Continue));
    }

    #[test]
    fn test_cooldown_prevents_spam() {
        let mut monitor = TokenUsageMonitor::default_thresholds();

        let mut state = SessionState::new("test".to_string(), None, Utc::now());
        state.model = Some("claude-3-5-sonnet-20241022".to_string());
        state.current_usage = ContextWindowUsage::from_raw(160_000, 0, 0, 10_000);

        let event = create_token_usage_event(160_000, 10_000);
        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        // First call should warn
        let result1 = monitor.handle(ctx).unwrap();
        assert!(matches!(result1, Reaction::Warn(_)));

        // Second immediate call should not warn (cooldown)
        let result2 = monitor.handle(ctx).unwrap();
        assert!(matches!(result2, Reaction::Continue));
    }
}
