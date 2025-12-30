use agtrace_engine::ContextWindowUsage;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SessionState {
    pub session_id: String,
    pub project_root: Option<PathBuf>,
    pub log_path: Option<PathBuf>,
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

    /// Get total tokens as i32 (legacy compatibility)
    /// Prefer using `total_tokens()` for type-safe token counting
    pub fn total_context_window_tokens(&self) -> i32 {
        self.total_tokens().as_u64() as i32
    }

    /// Get total tokens as type-safe TokenCount
    pub fn total_tokens(&self) -> agtrace_engine::TokenCount {
        self.current_usage.total_tokens()
    }

    /// Get context limit as type-safe ContextLimit
    pub fn context_limit(&self) -> Option<agtrace_engine::ContextLimit> {
        self.context_window_limit
            .map(agtrace_engine::ContextLimit::new)
    }

    pub fn validate_tokens(&self, model_limit: Option<u64>) -> Result<(), String> {
        if self.current_usage.fresh_input.0 < 0
            || self.current_usage.output.0 < 0
            || self.current_usage.cache_creation.0 < 0
            || self.current_usage.cache_read.0 < 0
        {
            return Err("Negative token count detected".to_string());
        }

        let total = self.total_tokens();
        if let Some(limit) = model_limit
            && total.as_u64() > limit
        {
            return Err(format!(
                "Token count {} exceeds model limit {}",
                total.as_u64(),
                limit
            ));
        }

        Ok(())
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
        assert_eq!(state.total_context_window_tokens(), 150);

        state.current_usage = ContextWindowUsage::from_raw(10, 0, 1000, 60);
        assert_eq!(state.total_input_side_tokens(), 1010);
        assert_eq!(state.total_output_side_tokens(), 60);
        assert_eq!(state.total_context_window_tokens(), 1070);
    }

    #[test]
    fn test_validate_tokens_success() {
        let mut state = SessionState::new("test-id".to_string(), None, None, Utc::now());
        state.current_usage = ContextWindowUsage::from_raw(1000, 2000, 10000, 500);
        assert!(state.validate_tokens(Some(200_000)).is_ok());
    }

    #[test]
    fn test_validate_tokens_exceeds_limit() {
        let mut state = SessionState::new("test-id".to_string(), None, None, Utc::now());
        state.current_usage = ContextWindowUsage::from_raw(100_000, 0, 0, 150_000);
        let result = state.validate_tokens(Some(200_000));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds model limit"));
    }

    #[test]
    fn test_validate_tokens_negative() {
        let mut state = SessionState::new("test-id".to_string(), None, None, Utc::now());
        state.current_usage = ContextWindowUsage::from_raw(-100, 0, 0, 0);

        let result = state.validate_tokens(None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Negative token count detected");
    }
}
