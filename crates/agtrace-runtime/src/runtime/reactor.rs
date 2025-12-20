use crate::domain::SessionState;
use agtrace_types::AgentEvent;
use anyhow::Result;

#[derive(Debug, Clone)]
pub enum Reaction {
    Continue,
    Warn(String),
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
    use chrono::Utc;

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
        let session_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000002").unwrap();

        AgentEvent {
            id,
            session_id,
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
