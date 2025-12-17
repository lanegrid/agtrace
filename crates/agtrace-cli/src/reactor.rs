use crate::token_usage::ContextWindowUsage;
use agtrace_types::v2::AgentEvent;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// Reaction returned by reactors to instruct the main loop
#[derive(Debug, Clone)]
pub enum Reaction {
    /// Continue processing normally
    Continue,

    /// Warning level - logs warning but continues
    Warn(String),

    /// Intervention request - requires action from main loop
    Intervene { reason: String, severity: Severity },
}

/// Severity level for interventions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Low severity - desktop notification or console alert
    Notification,

    /// High severity - terminate the agent process (future: v0.2.0)
    Kill,
}

/// Lightweight session state metadata for reactor context
/// This is NOT the full AgentSession, but a summary of key metrics
///
/// NOTE: Token fields are SNAPSHOTS, not cumulative totals
///
/// Why snapshots?
/// - LLMs receive the full conversation history on EVERY turn
/// - TokenUsage events report the current turn's token breakdown, not deltas
/// - Example: Turn 1 uses 100 input tokens, Turn 2 uses 150 input tokens
///   → Turn 2's event shows input_tokens=150 (the full prompt size this turn),
///   NOT input_tokens=50 (incremental)
///
/// This is critical for:
/// 1. Accurate context window tracking (latest snapshot = current usage)
/// 2. Prompt caching visibility (cache_read shows reused tokens each turn)
/// 3. Rate limit calculations (must use current snapshot, not accumulated total)
///
/// See: e156a8e "fix fundamental misunderstanding - use snapshots not cumulative totals"
#[derive(Debug, Clone)]
pub struct SessionState {
    /// Session/trace ID
    pub session_id: String,

    /// Project root path
    pub project_root: Option<PathBuf>,

    /// Session start time
    pub start_time: DateTime<Utc>,

    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,

    /// Model name (e.g., "claude-3-5-sonnet-20241022")
    pub model: Option<String>,

    /// Explicit context window limit reported by the provider (if available)
    pub context_window_limit: Option<u64>,

    /// Current turn's context window usage (type-safe token tracking)
    /// This is a SNAPSHOT, not cumulative
    ///
    /// The ContextWindowUsage type makes it IMPOSSIBLE to forget including
    /// cache_read tokens - the compiler enforces correct calculation.
    pub current_usage: ContextWindowUsage,

    /// Current turn's reasoning tokens (extended thinking)
    /// This is a SNAPSHOT, not cumulative
    pub current_reasoning_tokens: i32,

    /// Consecutive error count (reset on success)
    pub error_count: u32,

    /// Total number of events processed
    pub event_count: usize,

    /// Number of turns (user inputs)
    pub turn_count: usize,
}

impl SessionState {
    /// Create initial state from first event
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

    /// Total tokens on input side for CURRENT turn (what LLM receives this turn)
    /// Includes: fresh input + cache creation + cache read.
    ///
    /// NOTE: cache_read tokens DO consume the context window.
    /// The type system guarantees cache_read is always included.
    pub fn total_input_side_tokens(&self) -> i32 {
        self.current_usage.input_side_total()
    }

    /// Total tokens on output side for CURRENT turn
    pub fn total_output_side_tokens(&self) -> i32 {
        self.current_usage.output_side_total()
    }

    /// Total context window usage for CURRENT turn (input + output)
    /// This represents what's currently in the context window.
    /// The type system guarantees cache_read is always included.
    pub fn total_context_window_tokens(&self) -> i32 {
        self.current_usage.total()
    }

    /// Validate token counts are reasonable for current turn
    pub fn validate_tokens(&self, model_limit: Option<u64>) -> Result<(), String> {
        let total = self.total_context_window_tokens();

        // Check for negative tokens (should never happen)
        if total < 0
            || self.current_usage.fresh_input.0 < 0
            || self.current_usage.output.0 < 0
            || self.current_usage.cache_creation.0 < 0
            || self.current_usage.cache_read.0 < 0
        {
            return Err("Negative token count detected".to_string());
        }

        // Check if total exceeds model limit (should warn if close)
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

/// Context passed to reactors for each event
#[derive(Clone, Copy)]
pub struct ReactorContext<'a> {
    /// Current event (trigger)
    pub event: &'a AgentEvent,

    /// Session state snapshot (background context)
    pub state: &'a SessionState,
}

/// Reactor trait - pluggable event handlers
pub trait Reactor: Send {
    /// Reactor name for debugging
    fn name(&self) -> &str;

    /// Handle an event and return a reaction
    /// This is called synchronously for each event in the main loop
    fn handle(&mut self, ctx: ReactorContext) -> Result<Reaction>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_types::v2::{AgentEvent, EventPayload, UserPayload};
    use chrono::Utc;

    /// Mock reactor for testing
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
    fn test_reactor_returns_intervene() {
        let mut reactor = MockReactor::new(
            "test",
            vec![Reaction::Intervene {
                reason: "test alert".to_string(),
                severity: Severity::Notification,
            }],
        );
        let event = create_test_event();
        let state = create_test_state();
        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = reactor.handle(ctx).unwrap();
        match result {
            Reaction::Intervene { reason, severity } => {
                assert_eq!(reason, "test alert");
                assert_eq!(severity, Severity::Notification);
            }
            _ => panic!("Expected Intervene reaction"),
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

        // Simulate Turn 1: 100 fresh input, 50 output
        state.current_usage = ContextWindowUsage::from_raw(100, 0, 0, 50);

        assert_eq!(state.total_input_side_tokens(), 100);
        assert_eq!(state.total_output_side_tokens(), 50);
        assert_eq!(state.total_context_window_tokens(), 150);

        // Simulate Turn 2: 10 fresh, 1000 from cache, 60 output
        // (Turn 1's 100 tokens are now cached and read back)
        state.current_usage = ContextWindowUsage::from_raw(10, 0, 1000, 60);

        // Context window should be based on Turn 2 only
        // Note: cache_read DOES consume context window (cheaper billing, but still processed)
        assert_eq!(state.total_input_side_tokens(), 1010); // 10 + 0 + 1000
        assert_eq!(state.total_output_side_tokens(), 60);
        assert_eq!(state.total_context_window_tokens(), 1070);
    }

    #[test]
    fn test_validate_tokens_success() {
        let mut state = SessionState::new("test-id".to_string(), None, Utc::now());
        state.current_usage = ContextWindowUsage::from_raw(1000, 2000, 10000, 500);

        // Should pass - total 13500 (1000+2000+10000+500) is under 200k limit
        assert!(state.validate_tokens(Some(200_000)).is_ok());
    }

    #[test]
    fn test_validate_tokens_exceeds_limit() {
        let mut state = SessionState::new("test-id".to_string(), None, Utc::now());
        state.current_usage = ContextWindowUsage::from_raw(100_000, 0, 0, 150_000);

        // Should fail - total 250k exceeds 200k limit
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

        // Should be Copy-able
        let ctx2 = ctx;
        let _ctx3 = ctx; // Can still use ctx after copy

        assert_eq!(ctx2.state.session_id, state.session_id);
    }
}
