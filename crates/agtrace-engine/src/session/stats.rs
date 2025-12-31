use super::types::*;
use agtrace_types::ContextWindowUsage;
use chrono::{DateTime, Utc};

pub fn calculate_session_stats(
    turns: &[AgentTurn],
    start_time: DateTime<Utc>,
    end_time: Option<DateTime<Utc>>,
) -> SessionStats {
    let total_turns = turns.len();
    let duration_seconds = end_time
        .map(|end| (end - start_time).num_seconds())
        .unwrap_or(0);
    let usage = turns
        .iter()
        .map(|t| t.stats.usage)
        .fold(ContextWindowUsage::default(), |acc, u| acc + u);

    SessionStats {
        total_turns,
        duration_seconds,
        usage,
    }
}

pub fn calculate_turn_stats(steps: &[AgentStep], turn_start: DateTime<Utc>) -> TurnStats {
    let step_count = steps.len();
    let duration_ms = steps
        .last()
        .map(|last| (last.timestamp - turn_start).num_milliseconds())
        .unwrap_or(0);

    // Use last-wins semantics: steps report cumulative context window snapshots
    // during streaming, so we take the final snapshot as the turn's usage
    let usage = steps
        .iter()
        .rev() // Start from the end
        .find_map(|s| s.usage.as_ref())
        .copied()
        .unwrap_or_default();

    TurnStats {
        duration_ms,
        step_count,
        usage,
    }
}

/// Convert TokenUsagePayload to ContextWindowUsage and merge into target
pub fn merge_usage(
    target: &mut Option<ContextWindowUsage>,
    source: &agtrace_types::TokenUsagePayload,
) {
    // Convert normalized TokenUsagePayload to ContextWindowUsage
    // Note: cache_creation is not tracked separately in the new schema
    let source_usage = ContextWindowUsage::from_raw(
        source.input.uncached as i32,
        0, // cache_creation - not separately tracked
        source.input.cached as i32,
        source.output.total() as i32,
    );

    // Replace target with source (last wins semantics for cumulative context window reporting)
    *target = Some(source_usage);
}

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_types::{TokenInput, TokenOutput, TokenUsagePayload};

    #[test]
    fn test_merge_usage_with_none() {
        let mut target = None;
        let source = TokenUsagePayload::new(
            TokenInput::new(0, 100),    // 0 cached, 100 uncached
            TokenOutput::new(50, 0, 0), // 50 generated, 0 reasoning, 0 tool
        );

        merge_usage(&mut target, &source);

        let result = target.unwrap();
        assert_eq!(result.fresh_input.0, 100);
        assert_eq!(result.output.0, 50);
        assert_eq!(result.cache_creation.0, 0);
        assert_eq!(result.cache_read.0, 0);
    }

    #[test]
    fn test_merge_usage_replaces_with_latest() {
        let mut target = Some(ContextWindowUsage::from_raw(100, 0, 0, 50));
        let source = TokenUsagePayload::new(
            TokenInput::new(0, 200),     // 0 cached, 200 uncached
            TokenOutput::new(100, 0, 0), // 100 generated, 0 reasoning, 0 tool
        );

        merge_usage(&mut target, &source);

        let result = target.unwrap();
        assert_eq!(result.fresh_input.0, 200);
        assert_eq!(result.output.0, 100);
        assert_eq!(result.cache_creation.0, 0);
        assert_eq!(result.cache_read.0, 0);
    }

    #[test]
    fn test_merge_usage_with_cache() {
        let mut target = Some(ContextWindowUsage::from_raw(100, 10, 20, 50));
        let source = TokenUsagePayload::new(
            TokenInput::new(40, 200),    // 40 cached, 200 uncached
            TokenOutput::new(70, 30, 0), // 70 generated, 30 reasoning, 0 tool
        );

        merge_usage(&mut target, &source);

        let result = target.unwrap();
        assert_eq!(result.fresh_input.0, 200);
        assert_eq!(result.output.0, 100); // 70 + 30 + 0
        assert_eq!(result.cache_creation.0, 0); // not tracked in new schema
        assert_eq!(result.cache_read.0, 40);
    }
}
