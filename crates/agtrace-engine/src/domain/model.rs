use crate::ContextWindowUsage;
use crate::context::{ContextEvidence, resolve};
use agtrace_types::{AgentEvent, ContextWindow, ModelCatalog, Provider};
use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// Real-time session state for live monitoring and watch operations.
///
/// Tracks cumulative metrics, token usage, and context window state
/// as events stream in. Used by the watch service to provide live updates.
#[derive(Debug, Clone)]
pub struct SessionState {
    /// Session UUID.
    pub session_id: String,
    /// Project root directory, if known.
    pub project_root: Option<PathBuf>,
    /// Path to the session's log file.
    pub log_path: Option<PathBuf>,
    /// Session start timestamp.
    pub start_time: DateTime<Utc>,
    /// Timestamp of the most recent event.
    pub last_activity: DateTime<Utc>,
    /// Model name/ID being used in this session.
    pub model: Option<String>,
    /// Provider of the watched agent (from the events' agent id).
    pub provider: Option<Provider>,
    /// Context-window evidence folded from the watched agent's events.
    pub context: ContextEvidence,
    /// Current cumulative token usage.
    pub current_usage: ContextWindowUsage,
    /// Current cumulative reasoning tokens (o1-style extended thinking).
    pub current_reasoning_tokens: i32,
    /// Count of errors encountered so far.
    pub error_count: u32,
    /// Total number of events processed.
    pub event_count: usize,
    /// Total number of user turns.
    pub turn_count: usize,
}

impl SessionState {
    pub fn new(
        session_id: String,
        project_root: Option<PathBuf>,
        log_path: Option<PathBuf>,
        start_time: DateTime<Utc>,
    ) -> Self {
        Self {
            session_id,
            project_root,
            log_path,
            start_time,
            last_activity: start_time,
            model: None,
            provider: None,
            context: ContextEvidence::default(),
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

    /// Get total tokens as type-safe TokenCount
    pub fn total_tokens(&self) -> crate::TokenCount {
        self.current_usage.total_tokens()
    }

    /// Fold one event of the watched agent into the state.
    ///
    /// The single state-update path shared by `watch` (TUI / console) and `demo`.
    pub fn apply_event(&mut self, event: &AgentEvent) {
        self.last_activity = event.timestamp;
        self.event_count += 1;
        self.provider = Some(event.agent.provider());
        self.context.apply(event);

        let updates = crate::extract_state_updates(event);
        if updates.is_new_turn {
            self.turn_count += 1;
        }
        if let Some(usage) = updates.usage {
            self.current_usage = usage;
        }
        if let Some(reasoning) = updates.reasoning_tokens {
            self.current_reasoning_tokens = reasoning;
        }
        // Latest model wins (not first-wins).
        if let Some(model) = updates.model {
            self.model = Some(model);
        }
    }

    /// Resolve the context window of the watched agent (design §3).
    pub fn context_window(&self, catalog: &dyn ModelCatalog) -> Option<ContextWindow> {
        resolve(self.provider?, &self.context, catalog)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_initialization() {
        let state = SessionState::new("test-id".to_string(), None, None, Utc::now());

        assert_eq!(state.session_id, "test-id");
        assert!(state.current_usage.is_empty());
        assert_eq!(state.current_reasoning_tokens, 0);
        assert_eq!(state.error_count, 0);
        assert_eq!(state.event_count, 0);
        assert_eq!(state.turn_count, 0);
    }

    #[test]
    fn test_session_state_token_snapshot() {
        let mut state = SessionState::new("test-id".to_string(), None, None, Utc::now());

        state.current_usage = ContextWindowUsage::from_raw(100, 0, 0, 50);
        assert_eq!(state.total_input_side_tokens(), 100);
        assert_eq!(state.total_output_side_tokens(), 50);
        assert_eq!(state.total_tokens(), crate::TokenCount::new(150));

        state.current_usage = ContextWindowUsage::from_raw(10, 0, 1000, 60);
        assert_eq!(state.total_input_side_tokens(), 1010);
        assert_eq!(state.total_output_side_tokens(), 60);
        assert_eq!(state.total_tokens(), crate::TokenCount::new(1070));
    }
}
