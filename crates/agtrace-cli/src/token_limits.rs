use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TokenLimit {
    pub input_limit: u64,
    pub output_limit: u64,
    pub total_limit: u64,
}

impl TokenLimit {
    pub fn new(input_limit: u64, output_limit: u64, total_limit: u64) -> Self {
        Self {
            input_limit,
            output_limit,
            total_limit,
        }
    }
}

pub struct TokenLimits {
    limits: HashMap<String, TokenLimit>,
}

impl TokenLimits {
    pub fn new() -> Self {
        let mut limits = HashMap::new();

        // Claude models
        limits.insert(
            "claude-3-5-sonnet-20241022".to_string(),
            TokenLimit::new(200_000, 8_192, 200_000),
        );
        limits.insert(
            "claude-3-5-sonnet-20240620".to_string(),
            TokenLimit::new(200_000, 8_192, 200_000),
        );
        limits.insert(
            "claude-3-opus-20240229".to_string(),
            TokenLimit::new(200_000, 4_096, 200_000),
        );
        limits.insert(
            "claude-3-haiku-20240307".to_string(),
            TokenLimit::new(200_000, 4_096, 200_000),
        );
        limits.insert(
            "claude-sonnet-4-5-20250929".to_string(),
            TokenLimit::new(200_000, 8_192, 200_000),
        );

        // Codex models (using default limits)
        limits.insert(
            "gpt-4o".to_string(),
            TokenLimit::new(128_000, 16_384, 128_000),
        );
        limits.insert(
            "gpt-4o-mini".to_string(),
            TokenLimit::new(128_000, 16_384, 128_000),
        );
        limits.insert(
            "gpt-4-turbo".to_string(),
            TokenLimit::new(128_000, 4_096, 128_000),
        );

        // Gemini models
        limits.insert(
            "gemini-1.5-pro".to_string(),
            TokenLimit::new(2_000_000, 8_192, 2_000_000),
        );
        limits.insert(
            "gemini-1.5-flash".to_string(),
            TokenLimit::new(1_000_000, 8_192, 1_000_000),
        );
        limits.insert(
            "gemini-2.0-flash-exp".to_string(),
            TokenLimit::new(1_000_000, 8_192, 1_000_000),
        );

        Self { limits }
    }

    pub fn get_limit(&self, model: &str) -> Option<&TokenLimit> {
        // Try exact match first
        if let Some(limit) = self.limits.get(model) {
            return Some(limit);
        }

        // Try prefix match for model variants
        for (key, limit) in &self.limits {
            if model.starts_with(key) || key.starts_with(model) {
                return Some(limit);
            }
        }

        None
    }

    /// Get usage percentage from SessionState
    /// Returns (input_pct, output_pct, total_pct)
    ///
    /// This is the safe method that correctly calculates percentages
    /// including cache tokens. Prefer this over the raw token method.
    pub fn get_usage_percentage_from_state(
        &self,
        state: &crate::reactor::SessionState,
    ) -> Option<(f64, f64, f64)> {
        let model = state.model.as_ref()?;
        let limit = self.get_limit(model)?;

        let input_side = state.total_input_side_tokens() as u64;
        let output_side = state.total_output_side_tokens() as u64;
        let total = state.total_context_window_tokens() as u64;

        let input_pct = (input_side as f64 / limit.total_limit as f64) * 100.0;
        let output_pct = (output_side as f64 / limit.total_limit as f64) * 100.0;
        let total_pct = (total as f64 / limit.total_limit as f64) * 100.0;

        Some((input_pct, output_pct, total_pct))
    }

    /// DEPRECATED: Use get_usage_percentage_from_state instead
    ///
    /// This method does NOT include cache tokens in the calculation,
    /// which leads to severe underreporting of context window usage.
    #[deprecated(
        since = "0.1.1",
        note = "Does not include cache tokens. Use get_usage_percentage_from_state instead"
    )]
    pub fn get_usage_percentage(
        &self,
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
    ) -> Option<(f64, f64, f64)> {
        let limit = self.get_limit(model)?;

        let input_pct = (input_tokens as f64 / limit.input_limit as f64) * 100.0;
        let output_pct = (output_tokens as f64 / limit.output_limit as f64) * 100.0;
        let total_pct = ((input_tokens + output_tokens) as f64 / limit.total_limit as f64) * 100.0;

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

    #[test]
    fn test_get_limit_exact_match() {
        let limits = TokenLimits::new();
        let limit = limits.get_limit("claude-3-5-sonnet-20241022");
        assert!(limit.is_some());
        assert_eq!(limit.unwrap().total_limit, 200_000);
    }

    #[test]
    fn test_get_limit_prefix_match() {
        let limits = TokenLimits::new();
        let limit = limits.get_limit("claude-3-5-sonnet");
        assert!(limit.is_some());
    }

    #[test]
    fn test_get_usage_percentage() {
        let limits = TokenLimits::new();
        let (input_pct, output_pct, total_pct) = limits
            .get_usage_percentage("claude-3-5-sonnet-20241022", 100_000, 4_000)
            .unwrap();

        assert_eq!(input_pct, 50.0);
        assert!((output_pct - 48.828125).abs() < 0.01);
        assert_eq!(total_pct, 52.0);
    }

    #[test]
    fn test_unknown_model() {
        let limits = TokenLimits::new();
        let result = limits.get_limit("unknown-model");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_usage_percentage_from_state() {
        use crate::reactor::SessionState;
        use chrono::Utc;

        let limits = TokenLimits::new();
        let mut state = SessionState::new("test".to_string(), None, Utc::now());
        state.model = Some("claude-3-5-sonnet-20241022".to_string());
        state.total_input_tokens = 1000;
        state.total_output_tokens = 500;
        state.cache_creation_tokens = 2000;
        state.cache_read_tokens = 10000;

        let (input_pct, output_pct, total_pct) =
            limits.get_usage_percentage_from_state(&state).unwrap();

        // Total input side: 1000 + 2000 + 10000 = 13000
        // Total output side: 500
        // Total context window: 13500
        // Limit: 200000
        assert_eq!(input_pct, 6.5); // 13000 / 200000 * 100
        assert_eq!(output_pct, 0.25); // 500 / 200000 * 100
        assert_eq!(total_pct, 6.75); // 13500 / 200000 * 100
    }

    #[test]
    fn test_get_usage_percentage_from_state_no_cache() {
        use crate::reactor::SessionState;
        use chrono::Utc;

        let limits = TokenLimits::new();
        let mut state = SessionState::new("test".to_string(), None, Utc::now());
        state.model = Some("claude-3-5-sonnet-20241022".to_string());
        state.total_input_tokens = 100_000;
        state.total_output_tokens = 4_000;
        // No cache tokens

        let (input_pct, output_pct, total_pct) =
            limits.get_usage_percentage_from_state(&state).unwrap();

        assert_eq!(input_pct, 50.0); // 100000 / 200000 * 100
        assert_eq!(output_pct, 2.0); // 4000 / 200000 * 100
        assert_eq!(total_pct, 52.0); // 104000 / 200000 * 100
    }
}
