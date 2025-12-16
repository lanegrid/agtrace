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

    /// Total input tokens consumed
    pub total_input_tokens: i32,

    /// Total output tokens consumed
    pub total_output_tokens: i32,

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
            total_input_tokens: 0,
            total_output_tokens: 0,
            error_count: 0,
            event_count: 0,
            turn_count: 0,
        }
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
        assert_eq!(state.total_input_tokens, 0);
        assert_eq!(state.total_output_tokens, 0);
        assert_eq!(state.error_count, 0);
        assert_eq!(state.event_count, 0);
        assert_eq!(state.turn_count, 0);
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
