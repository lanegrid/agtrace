//! Extension traits for session analysis.
//!
//! These traits provide complex computation logic for session types
//! defined in agtrace-types, keeping the types crate lightweight and
//! focused on data structures while engine handles analysis logic.

use super::types::{AgentSession, AgentTurn, TurnMetrics};

/// Detect if context was compacted by analyzing origin and token drop.
///
/// Context compaction is detected when:
/// 1. Turn origin is ContextCompaction (authoritative), OR
/// 2. Tokens dropped by more than 50% (fallback for backward compatibility)
fn detect_context_compaction(turn: &AgentTurn, current_tokens: u32, prev_cumulative: u32) -> bool {
    // Primary: origin-based detection (authoritative)
    if turn.user.origin.is_context_compaction() {
        return true;
    }

    // Fallback: token-based detection for backward compatibility
    // (handles data assembled before origin was added)
    prev_cumulative > 0 && current_tokens < prev_cumulative / 2
}

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
            let turn_end_tokens = turn.cumulative_total_tokens(cumulative_total);

            // Detect if context was compacted (reset) during this turn
            let context_compacted =
                detect_context_compaction(turn, turn_end_tokens, cumulative_total);

            // Calculate prev_total, delta, and compaction_from based on compaction state
            let (prev_total, delta, compaction_from) = if context_compacted {
                // Context was reset: prev=0, delta=new baseline
                // Store the previous cumulative to show reduction (e.g., 150k → 30k)
                (0, turn_end_tokens, Some(cumulative_total))
            } else {
                // Normal case: additive
                let delta = turn_end_tokens.saturating_sub(cumulative_total);
                (cumulative_total, delta, None)
            };

            // Simplified: Last turn is always considered active for live watching
            // This avoids flickering caused by per-step status checks
            let is_active = idx == total_turns - 1;

            metrics.push(TurnMetrics {
                turn_index: idx,
                prev_total,
                delta,
                is_heavy: TurnMetrics::is_delta_heavy(delta, max_context),
                is_active,
                context_compacted,
                cumulative_total: turn_end_tokens,
                compaction_from,
            });

            cumulative_total = turn_end_tokens;
        }

        metrics
    }
}
