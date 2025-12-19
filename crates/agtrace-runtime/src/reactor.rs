use agtrace_engine::ContextWindowUsage;
use agtrace_types::AgentEvent;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// Reaction returned by reactors to instruct the main loop
#[derive(Debug, Clone)]
pub enum Reaction {
    Continue,
    Warn(String),
}

/// Lightweight session state metadata for reactor context
#[derive(Debug, Clone)]
pub struct SessionState {
    pub session_id: String,
    pub project_root: Option<PathBuf>,
    pub start_time: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub model: Option<String>,
    pub context_window_limit: Option<u64>,
    pub current_usage: ContextWindowUsage,
    pub current_reasoning_tokens: i32,
    pub error_count: u32,
    pub event_count: usize,
    pub turn_count: usize,
}

impl SessionState {
    pub fn new(
        session_id: String,
        project_root: Option<PathBuf>,
        start_time: DateTime<Utc>,
    ) -> Self {
        Self {
            session_id,
            project_root,
            start_time,
            last_activity: start_time,
            model: None,
            context_window_limit: None,
            current_usage: ContextWindowUsage::default(),
            current_reasoning_tokens: 0,
            error_count: 0,
            event_count: 0,
            turn_count: 0,
        }
    }

    pub fn total_input_side_tokens(&self) -> i32 {
        self.current_usage.input_tokens()
    }

    pub fn total_output_side_tokens(&self) -> i32 {
        self.current_usage.output_tokens()
    }

    pub fn total_context_window_tokens(&self) -> i32 {
        self.current_usage.context_window_tokens()
    }

    pub fn validate_tokens(&self, model_limit: Option<u64>) -> Result<(), String> {
        let total = self.total_context_window_tokens();

        if total < 0
            || self.current_usage.fresh_input.0 < 0
            || self.current_usage.output.0 < 0
            || self.current_usage.cache_creation.0 < 0
            || self.current_usage.cache_read.0 < 0
        {
            return Err("Negative token count detected".to_string());
        }

        if let Some(limit) = model_limit {
            if total as u64 > limit {
                return Err(format!(
                    "Token count {} exceeds model limit {}",
                    total, limit
                ));
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct ReactorContext<'a> {
    pub event: &'a AgentEvent,
    pub state: &'a SessionState,
}

pub trait Reactor: Send {
    fn name(&self) -> &str;
    fn handle(&mut self, ctx: ReactorContext) -> Result<Reaction>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_types::{EventPayload, UserPayload};

    struct MockReactor {
        name: String,
        reactions: Vec<Reaction>,
        call_count: usize,
    }

    impl MockReactor {
        fn new(name: &str, reactions: Vec<Reaction>) -> Self {
            Self {
                name: name.to_string(),
                reactions,
                call_count: 0,
            }
        }
    }

    impl Reactor for MockReactor {
        fn name(&self) -> &str {
            &self.name
        }

        fn handle(&mut self, _ctx: ReactorContext) -> Result<Reaction> {
            let reaction = self
                .reactions
                .get(self.call_count)
                .cloned()
                .unwrap_or(Reaction::Continue);
            self.call_count += 1;
            Ok(reaction)
        }
    }

    fn create_test_event() -> AgentEvent {
        use std::str::FromStr;
        let id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap();
        let trace_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000002").unwrap();

        AgentEvent {
            id,
            trace_id,
            parent_id: None,
            timestamp: Utc::now(),
            stream_id: agtrace_types::StreamId::Main,
            payload: EventPayload::User(UserPayload {
                text: "test".to_string(),
            }),
            metadata: None,
        }
    }

    fn create_test_state() -> SessionState {
        SessionState::new("test-session".to_string(), None, Utc::now())
    }

    #[test]
    fn test_reactor_returns_continue() {
        let mut reactor = MockReactor::new("test", vec![Reaction::Continue]);
        let event = create_test_event();
        let state = create_test_state();
        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = reactor.handle(ctx).unwrap();
        assert!(matches!(result, Reaction::Continue));
        assert_eq!(reactor.call_count, 1);
    }

    #[test]
    fn test_reactor_returns_warn() {
        let mut reactor =
            MockReactor::new("test", vec![Reaction::Warn("test warning".to_string())]);
        let event = create_test_event();
        let state = create_test_state();
        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = reactor.handle(ctx).unwrap();
        match result {
            Reaction::Warn(msg) => assert_eq!(msg, "test warning"),
            _ => panic!("Expected Warn reaction"),
        }
    }

    #[test]
    fn test_session_state_initialization() {
        let state = SessionState::new("test-id".to_string(), None, Utc::now());

        assert_eq!(state.session_id, "test-id");
        assert!(state.current_usage.is_empty());
        assert_eq!(state.current_reasoning_tokens, 0);
        assert_eq!(state.error_count, 0);
        assert_eq!(state.event_count, 0);
        assert_eq!(state.turn_count, 0);
    }

    #[test]
    fn test_session_state_token_snapshot() {
        let mut state = SessionState::new("test-id".to_string(), None, Utc::now());

        state.current_usage = ContextWindowUsage::from_raw(100, 0, 0, 50);
        assert_eq!(state.total_input_side_tokens(), 100);
        assert_eq!(state.total_output_side_tokens(), 50);
        assert_eq!(state.total_context_window_tokens(), 150);

        state.current_usage = ContextWindowUsage::from_raw(10, 0, 1000, 60);
        assert_eq!(state.total_input_side_tokens(), 1010);
        assert_eq!(state.total_output_side_tokens(), 60);
        assert_eq!(state.total_context_window_tokens(), 1070);
    }

    #[test]
    fn test_validate_tokens_success() {
        let mut state = SessionState::new("test-id".to_string(), None, Utc::now());
        state.current_usage = ContextWindowUsage::from_raw(1000, 2000, 10000, 500);
        assert!(state.validate_tokens(Some(200_000)).is_ok());
    }

    #[test]
    fn test_validate_tokens_exceeds_limit() {
        let mut state = SessionState::new("test-id".to_string(), None, Utc::now());
        state.current_usage = ContextWindowUsage::from_raw(100_000, 0, 0, 150_000);
        let result = state.validate_tokens(Some(200_000));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds model limit"));
    }

    #[test]
    fn test_validate_tokens_negative() {
        let mut state = SessionState::new("test-id".to_string(), None, Utc::now());
        state.current_usage = ContextWindowUsage::from_raw(-100, 0, 0, 0);

        let result = state.validate_tokens(None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Negative token count detected");
    }

    #[test]
    fn test_reactor_context_copy() {
        let event = create_test_event();
        let state = create_test_state();

        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let ctx2 = ctx;
        let _ctx3 = ctx;

        assert_eq!(ctx2.state.session_id, state.session_id);
    }
}
