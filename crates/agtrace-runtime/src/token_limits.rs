use crate::reactor::SessionState;

// NOTE: Architecture decision - Thin CLI layer
// This module is intentionally thin and delegates to agtrace_providers::token_limits.
// The CLI layer's responsibility is only to:
// 1. Provide a stable API for existing consumers (TokenUsageMonitor)
// 2. Apply resolution priority (runtime metadata > provider knowledge)
// 3. Bridge between provider types (ModelSpec) and CLI types (TokenLimit)
//
// All model knowledge lives in agtrace_providers to maintain separation of concerns.
// This design allows the providers crate to be reused by other tools without CLI dependencies.

#[derive(Debug, Clone)]
pub struct TokenLimit {
    pub total_limit: u64,
    /// Compaction buffer percentage (0-100)
    /// When input tokens exceed (100% - compaction_buffer_pct), compaction is triggered
    pub compaction_buffer_pct: f64,
}

impl TokenLimit {
    pub fn new(total_limit: u64, compaction_buffer_pct: f64) -> Self {
        // Validate compaction_buffer_pct is in valid range
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

    /// Calculate effective token limit after accounting for compaction buffer
    ///
    /// Example: 200K context window with 22.5% buffer = 155K effective limit
    /// This is the actual usable limit before compaction triggers.
    pub fn effective_limit(&self) -> u64 {
        if self.compaction_buffer_pct == 0.0 {
            // No buffer, full limit is usable
            return self.total_limit;
        }

        let usable_pct = 100.0 - self.compaction_buffer_pct;
        let effective = (self.total_limit as f64 * usable_pct / 100.0).floor() as u64;

        // Ensure we never return 0 (even with 99.9% buffer, there's some usable space)
        effective.max(1)
    }
}

pub struct TokenLimits;

impl TokenLimits {
    pub fn new() -> Self {
        Self
    }

    pub fn get_limit(&self, model: &str) -> Option<TokenLimit> {
        // NOTE: Why delegate to providers instead of maintaining our own lookup?
        // This ensures we have a single source of truth for model specifications.
        // Previously, this file contained hardcoded model limits, which led to:
        // - Duplication across provider modules and CLI
        // - Inconsistencies when providers were updated but CLI wasn't
        // - High maintenance burden (every new model required changes in multiple places)
        //
        // By delegating to providers, we get:
        // - Automatic updates when provider definitions change
        // - Longest prefix matching for resilient version handling
        // - Clean separation between "knowledge" and "usage"
        agtrace_providers::token_limits::resolve_model_limit(model)
            .map(|spec| TokenLimit::new(spec.max_tokens, spec.compaction_buffer_pct))
    }

    /// Get usage percentage from SessionState
    /// Returns (input_pct, output_pct, total_pct)
    ///
    /// This is the safe method that correctly calculates percentages
    /// including cache tokens. Prefer this over the raw token method.
    ///
    /// Resolution priority:
    /// 1. Runtime metadata (state.context_window_limit)
    /// 2. Provider knowledge (via longest prefix matching)
    ///
    /// NOTE: Why prioritize runtime metadata over provider knowledge?
    /// Runtime metadata comes directly from the LLM provider's log files and represents
    /// the actual context window limit that was in effect during that session. This is:
    /// - Always correct for that specific session (source of truth)
    /// - Handles special cases (e.g., beta extended context, custom enterprise limits)
    /// - Future-proof against provider changes we haven't updated yet
    ///
    /// Provider knowledge is a fallback heuristic for when metadata is unavailable.
    /// This follows the principle: "Trust the source, fallback to heuristics."
    pub fn get_usage_percentage_from_state(&self, state: &SessionState) -> Option<(f64, f64, f64)> {
        let limit_total = if let Some(l) = state.context_window_limit {
            // Priority 1: Runtime metadata from log files (always most accurate)
            l
        } else {
            // Priority 2: Provider knowledge via longest prefix matching (best-effort heuristic)
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
        use crate::reactor::SessionState;
        use agtrace_engine::ContextWindowUsage;
        use chrono::Utc;

        let limits = TokenLimits::new();
        let mut state = SessionState::new("test".to_string(), None, Utc::now());
        state.model = Some("claude-3-5-sonnet-20241022".to_string());
        state.current_usage = ContextWindowUsage::from_raw(1000, 2000, 10000, 500);

        let (input_pct, output_pct, total_pct) =
            limits.get_usage_percentage_from_state(&state).unwrap();

        // Total input side: 1000 + 2000 + 10000 = 13000 (cache read DOES consume context window)
        // Total output side: 500
        // Total context window: 13500
        // Limit: 200000
        let eps = 1e-6;
        assert!((input_pct - 6.5).abs() < eps); // 13000 / 200000 * 100
        assert!((output_pct - 0.25).abs() < eps); // 500 / 200000 * 100
        assert!((total_pct - 6.75).abs() < eps); // 13500 / 200000 * 100
    }

    #[test]
    fn test_get_usage_percentage_from_state_no_cache() {
        use crate::reactor::SessionState;
        use agtrace_engine::ContextWindowUsage;
        use chrono::Utc;

        let limits = TokenLimits::new();
        let mut state = SessionState::new("test".to_string(), None, Utc::now());
        state.model = Some("claude-3-5-sonnet-20241022".to_string());
        state.context_window_limit = Some(200_000); // direct limit should be preferred if set
        state.current_usage = ContextWindowUsage::from_raw(100_000, 0, 0, 4_000);
        // No cache tokens

        let (input_pct, output_pct, total_pct) =
            limits.get_usage_percentage_from_state(&state).unwrap();

        assert_eq!(input_pct, 50.0); // 100000 / 200000 * 100
        assert_eq!(output_pct, 2.0); // 4000 / 200000 * 100
        assert_eq!(total_pct, 52.0); // 104000 / 200000 * 100
    }

    #[test]
    fn test_effective_limit() {
        // Claude model with 22.5% buffer: 200K -> 155K effective
        let limit = TokenLimit::new(200_000, 22.5);
        assert_eq!(limit.effective_limit(), 155_000);

        // No buffer: full limit is usable
        let limit_no_buffer = TokenLimit::new(400_000, 0.0);
        assert_eq!(limit_no_buffer.effective_limit(), 400_000);

        // Edge case: very high buffer still returns non-zero
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
