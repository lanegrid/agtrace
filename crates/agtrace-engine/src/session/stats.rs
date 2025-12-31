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
    let cache_creation = source
        .details
        .as_ref()
        .and_then(|d| d.cache_creation_input_tokens)
        .unwrap_or(0);
    let cache_read = source
        .details
        .as_ref()
        .and_then(|d| d.cache_read_input_tokens)
        .unwrap_or(0);

    // Convert TokenUsagePayload to ContextWindowUsage
    let source_usage = ContextWindowUsage::from_raw(
        source.input_tokens,
        cache_creation,
        cache_read,
        source.output_tokens,
    );

    // Replace target with source (last wins semantics for cumulative context window reporting)
    *target = Some(source_usage);
}

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_types::{TokenUsageDetails, TokenUsagePayload};

    #[test]
    fn test_merge_usage_with_none() {
        let mut target = None;
        let source = TokenUsagePayload {
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
            details: None,
        };

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
        let source = TokenUsagePayload {
            input_tokens: 200,
            output_tokens: 100,
            total_tokens: 300,
            details: None,
        };

        merge_usage(&mut target, &source);

        let result = target.unwrap();
        assert_eq!(result.fresh_input.0, 200);
        assert_eq!(result.output.0, 100);
        assert_eq!(result.cache_creation.0, 0);
        assert_eq!(result.cache_read.0, 0);
    }

    #[test]
    fn test_merge_usage_with_details() {
        let mut target = Some(ContextWindowUsage::from_raw(100, 10, 20, 50));
        let source = TokenUsagePayload {
            input_tokens: 200,
            output_tokens: 100,
            total_tokens: 300,
            details: Some(TokenUsageDetails {
                cache_creation_input_tokens: Some(15),
                cache_read_input_tokens: Some(40),
                reasoning_output_tokens: Some(30),
            }),
        };

        merge_usage(&mut target, &source);

        let result = target.unwrap();
        assert_eq!(result.fresh_input.0, 200);
        assert_eq!(result.output.0, 100);
        assert_eq!(result.cache_creation.0, 15);
        assert_eq!(result.cache_read.0, 40);
    }
}
