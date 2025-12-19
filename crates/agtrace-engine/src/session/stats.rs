use super::types::*;
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
    let total_tokens: i64 = turns.iter().map(|t| t.stats.total_tokens as i64).sum();

    SessionStats {
        total_turns,
        duration_seconds,
        total_tokens,
    }
}

pub fn calculate_turn_stats(steps: &[AgentStep], turn_start: DateTime<Utc>) -> TurnStats {
    let step_count = steps.len();
    let duration_ms = steps
        .last()
        .map(|last| (last.timestamp - turn_start).num_milliseconds())
        .unwrap_or(0);
    let total_tokens: i32 = steps
        .iter()
        .filter_map(|s| s.usage.as_ref())
        .map(|u| u.total_tokens)
        .sum();

    TurnStats {
        duration_ms,
        step_count,
        total_tokens,
    }
}

pub fn merge_usage(
    target: &mut Option<agtrace_types::TokenUsagePayload>,
    source: &agtrace_types::TokenUsagePayload,
) {
    if let Some(current) = target {
        current.input_tokens = current.input_tokens.max(source.input_tokens);
        current.output_tokens = current.output_tokens.max(source.output_tokens);
        current.total_tokens = current.total_tokens.max(source.total_tokens);

        if let (Some(d1), Some(d2)) = (&mut current.details, &source.details) {
            if let (Some(v1), Some(v2)) = (d1.cache_read_input_tokens, d2.cache_read_input_tokens) {
                d1.cache_read_input_tokens = Some(v1.max(v2));
            }
            if let (Some(v1), Some(v2)) = (d1.reasoning_output_tokens, d2.reasoning_output_tokens) {
                d1.reasoning_output_tokens = Some(v1.max(v2));
            }
        } else if current.details.is_none() {
            current.details = source.details.clone();
        }
    } else {
        *target = Some(source.clone());
    }
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

        assert_eq!(target.unwrap().total_tokens, 150);
    }

    #[test]
    fn test_merge_usage_with_existing() {
        let mut target = Some(TokenUsagePayload {
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
            details: None,
        });
        let source = TokenUsagePayload {
            input_tokens: 200,
            output_tokens: 100,
            total_tokens: 300,
            details: None,
        };

        merge_usage(&mut target, &source);

        let result = target.unwrap();
        assert_eq!(result.input_tokens, 200);
        assert_eq!(result.output_tokens, 100);
        assert_eq!(result.total_tokens, 300);
    }

    #[test]
    fn test_merge_usage_with_details() {
        let mut target = Some(TokenUsagePayload {
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
            details: Some(TokenUsageDetails {
                cache_creation_input_tokens: Some(10),
                cache_read_input_tokens: Some(20),
                reasoning_output_tokens: Some(10),
            }),
        });
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
        assert_eq!(result.total_tokens, 300);
        let details = result.details.unwrap();
        assert_eq!(details.cache_read_input_tokens, Some(40));
        assert_eq!(details.reasoning_output_tokens, Some(30));
    }
}
