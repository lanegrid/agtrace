//! Extension traits for session analysis.
//!
//! These traits provide complex computation logic for session types
//! defined in agtrace-types, keeping the types crate lightweight and
//! focused on data structures while engine handles analysis logic.

use super::types::{AgentSession, TurnMetrics};

/// Extension trait for `AgentSession` providing analysis and metrics computation.
pub trait SessionAnalysisExt {
    /// Compute presentation metrics for all turns
    fn compute_turn_metrics(&self, max_context: Option<u32>) -> Vec<TurnMetrics>;
}

impl SessionAnalysisExt for AgentSession {
    fn compute_turn_metrics(&self, max_context: Option<u32>) -> Vec<TurnMetrics> {
        let mut cumulative_total = 0u32;
        let mut metrics = Vec::new();
        let total_turns = self.turns.len();

        for (idx, turn) in self.turns.iter().enumerate() {
            let turn_end_cumulative = turn.cumulative_total_tokens(cumulative_total);
            let delta = turn_end_cumulative.saturating_sub(cumulative_total);
            let prev_total = cumulative_total;

            // Last turn is always active during streaming to avoid flicker
            // when steps transition between InProgress and Done
            let is_active = idx == total_turns.saturating_sub(1);

            metrics.push(TurnMetrics {
                turn_index: idx,
                prev_total,
                delta,
                is_heavy: TurnMetrics::is_delta_heavy(delta, max_context),
                is_active,
            });

            cumulative_total = turn_end_cumulative;
        }

        metrics
    }
}
