use crate::domain::model::SessionState;

#[derive(Debug, Clone)]
pub struct TokenLimit {
    pub total_limit: u64,
    pub compaction_buffer_pct: f64,
}

impl TokenLimit {
    pub fn new(total_limit: u64, compaction_buffer_pct: f64) -> Self {
        assert!(
            (0.0..=100.0).contains(&compaction_buffer_pct),
            "compaction_buffer_pct must be in range 0-100, got: {}",
            compaction_buffer_pct
        );

        Self {
            total_limit,
            compaction_buffer_pct,
        }
    }

    pub fn effective_limit(&self) -> u64 {
        if self.compaction_buffer_pct == 0.0 {
            return self.total_limit;
        }

        let usable_pct = 100.0 - self.compaction_buffer_pct;
        let effective = (self.total_limit as f64 * usable_pct / 100.0).floor() as u64;

        effective.max(1)
    }
}

pub struct TokenLimits;

impl TokenLimits {
    pub fn new() -> Self {
        Self
    }

    pub fn get_limit(&self, model: &str) -> Option<TokenLimit> {
        agtrace_providers::token_limits::resolve_model_limit(model)
            .map(|spec| TokenLimit::new(spec.max_tokens, spec.compaction_buffer_pct))
    }

    pub fn get_usage_percentage_from_state(&self, state: &SessionState) -> Option<(f64, f64, f64)> {
        let limit_total = if let Some(l) = state.context_window_limit {
            l
        } else {
            let model = state.model.as_ref()?;
            self.get_limit(model)?.total_limit
        };

        let input_side = state.total_input_side_tokens() as u64;
        let output_side = state.total_output_side_tokens() as u64;
        let total = state.total_context_window_tokens() as u64;

        let input_pct = (input_side as f64 / limit_total as f64) * 100.0;
        let output_pct = (output_side as f64 / limit_total as f64) * 100.0;
        let total_pct = (total as f64 / limit_total as f64) * 100.0;

        Some((input_pct, output_pct, total_pct))
    }
}

impl Default for TokenLimits {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_engine::ContextWindowUsage;
    use chrono::Utc;

    #[test]
    fn test_get_limit_exact_match() {
        let limits = TokenLimits::new();
        let limit = limits.get_limit("claude-3-5-sonnet-20241022");
        assert!(limit.is_some());
        let limit = limit.unwrap();
        assert_eq!(limit.total_limit, 200_000);
        assert_eq!(limit.compaction_buffer_pct, 22.5);
        assert_eq!(limit.effective_limit(), 155_000);
    }

    #[test]
    fn test_get_limit_prefix_match() {
        let limits = TokenLimits::new();
        let limit = limits.get_limit("claude-3-5-sonnet");
        assert!(limit.is_some());
    }

    #[test]
    fn test_unknown_model() {
        let limits = TokenLimits::new();
        let result = limits.get_limit("unknown-model");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_usage_percentage_from_state() {
        let limits = TokenLimits::new();
        let mut state = SessionState::new("test".to_string(), None, None, Utc::now());
        state.model = Some("claude-3-5-sonnet-20241022".to_string());
        state.current_usage = ContextWindowUsage::from_raw(1000, 2000, 10000, 500);

        let (input_pct, output_pct, total_pct) =
            limits.get_usage_percentage_from_state(&state).unwrap();

        let eps = 1e-6;
        assert!((input_pct - 6.5).abs() < eps);
        assert!((output_pct - 0.25).abs() < eps);
        assert!((total_pct - 6.75).abs() < eps);
    }

    #[test]
    fn test_get_usage_percentage_from_state_no_cache() {
        let limits = TokenLimits::new();
        let mut state = SessionState::new("test".to_string(), None, None, Utc::now());
        state.model = Some("claude-3-5-sonnet-20241022".to_string());
        state.context_window_limit = Some(200_000);
        state.current_usage = ContextWindowUsage::from_raw(100_000, 0, 0, 4_000);

        let (input_pct, output_pct, total_pct) =
            limits.get_usage_percentage_from_state(&state).unwrap();

        assert_eq!(input_pct, 50.0);
        assert_eq!(output_pct, 2.0);
        assert_eq!(total_pct, 52.0);
    }

    #[test]
    fn test_effective_limit() {
        let limit = TokenLimit::new(200_000, 22.5);
        assert_eq!(limit.effective_limit(), 155_000);

        let limit_no_buffer = TokenLimit::new(400_000, 0.0);
        assert_eq!(limit_no_buffer.effective_limit(), 400_000);

        let limit_high_buffer = TokenLimit::new(1000, 99.0);
        assert_eq!(limit_high_buffer.effective_limit(), 10);
    }

    #[test]
    #[should_panic(expected = "compaction_buffer_pct must be in range 0-100")]
    fn test_invalid_buffer_pct_negative() {
        TokenLimit::new(200_000, -10.0);
    }

    #[test]
    #[should_panic(expected = "compaction_buffer_pct must be in range 0-100")]
    fn test_invalid_buffer_pct_over_100() {
        TokenLimit::new(200_000, 150.0);
    }
}
