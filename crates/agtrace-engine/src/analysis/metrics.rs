use crate::AgentSession;

#[derive(Debug, Clone)]
pub struct SessionMetrics {
    pub tool_calls_total: usize,
    pub tool_failures_total: usize,
    pub max_e2e_ms: u64,
    pub max_tool_ms: u64,
    pub missing_tool_pairs: usize,
    pub loop_signals: usize,
    pub longest_chain: usize,
}

pub fn compute_metrics(session: &AgentSession) -> SessionMetrics {
    let mut tool_calls_total = 0;
    let mut tool_failures_total = 0;
    let mut missing_tool_pairs = 0;
    let mut max_tool_ms = 0i64;
    let mut longest_chain = 0;
    let mut tool_counts_per_turn = Vec::new();

    for turn in &session.turns {
        let mut turn_tool_count = 0;
        for step in &turn.steps {
            turn_tool_count += step.tools.len();
            tool_calls_total += step.tools.len();

            for tool_exec in &step.tools {
                if tool_exec.is_error {
                    tool_failures_total += 1;
                }
                if tool_exec.result.is_none() {
                    missing_tool_pairs += 1;
                }
                if let Some(duration_ms) = tool_exec.duration_ms {
                    if duration_ms > max_tool_ms {
                        max_tool_ms = duration_ms;
                    }
                }
            }

            if step.tools.len() > longest_chain {
                longest_chain = step.tools.len();
            }
        }
        tool_counts_per_turn.push(turn_tool_count);
    }

    // Calculate max e2e time (turn duration)
    let max_e2e_ms = session
        .turns
        .iter()
        .map(|turn| turn.stats.duration_ms)
        .max()
        .unwrap_or(0);

    // Heuristic: Loop detection (same turn has > 5 tool calls)
    let loop_signals = tool_counts_per_turn.iter().filter(|&&c| c > 5).count();

    SessionMetrics {
        tool_calls_total,
        tool_failures_total,
        max_e2e_ms: max_e2e_ms as u64,
        max_tool_ms: max_tool_ms as u64,
        missing_tool_pairs,
        loop_signals,
        longest_chain,
    }
}

// Tests will be added after integration is complete and we can use real session data
