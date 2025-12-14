use agtrace_types::v2::{AgentEvent, EventPayload};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub session_id: String,
    pub score: u32,
    pub warnings: Vec<PatternWarning>,
    pub info: Vec<PatternInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatternWarning {
    pub pattern: String,
    pub count: usize,
    pub span: String,
    pub insight: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatternInfo {
    pub category: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum Detector {
    Loops,
    Apology,
    LazyTool,
    Zombie,
    LintPingPong,
}

impl FromStr for Detector {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "loops" => Ok(Detector::Loops),
            "apology" => Ok(Detector::Apology),
            "lazy_tool" => Ok(Detector::LazyTool),
            "zombie" => Ok(Detector::Zombie),
            "lint_ping_pong" => Ok(Detector::LintPingPong),
            _ => Err(format!("Unknown detector: {}", s)),
        }
    }
}

impl Detector {
    pub fn all() -> Vec<Self> {
        vec![
            Detector::Loops,
            Detector::Apology,
            Detector::LazyTool,
            Detector::Zombie,
            Detector::LintPingPong,
        ]
    }
}

pub fn analyze(
    session_id: String,
    events: &[AgentEvent],
    detectors: Vec<Detector>,
) -> AnalysisReport {
    let mut warnings = Vec::new();
    let mut info_items = Vec::new();

    for detector in detectors {
        match detector {
            Detector::Loops => detect_loops(events, &mut warnings),
            Detector::Apology => detect_apologies(events, &mut warnings),
            Detector::LazyTool => detect_lazy_tools(events, &mut warnings),
            Detector::Zombie => detect_zombie_chains(events, &mut warnings),
            Detector::LintPingPong => detect_lint_ping_pong(events, &mut warnings),
        }
    }

    analyze_tool_usage(events, &mut info_items);

    let score = calculate_score(&warnings);

    AnalysisReport {
        session_id,
        score,
        warnings,
        info: info_items,
    }
}

fn detect_loops(events: &[AgentEvent], warnings: &mut Vec<PatternWarning>) {
    let mut i = 0;
    while i < events.len() {
        if let EventPayload::ToolCall(call) = &events[i].payload {
            let tool_name = &call.name;

            let mut loop_count = 0;
            let mut j = i;
            let start_ts = events[i].timestamp;
            let mut end_ts = start_ts;

            // Look ahead for repeated calls with errors
            while j < events.len().saturating_sub(1) {
                if let EventPayload::ToolCall(next_call) = &events[j].payload {
                    if next_call.name == *tool_name {
                        // Find corresponding result
                        if let Some(result_event) = events[j + 1..].iter().find(|e| {
                            if let EventPayload::ToolResult(r) = &e.payload {
                                r.tool_call_id == events[j].id
                            } else {
                                false
                            }
                        }) {
                            if let EventPayload::ToolResult(result) = &result_event.payload {
                                if result.is_error {
                                    loop_count += 1;
                                    end_ts = result_event.timestamp;
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                }
                j += 1;
            }

            if loop_count >= 2 {
                let span = format_time_span(start_ts, end_ts);
                warnings.push(PatternWarning {
                    pattern: "Loop Detected".to_string(),
                    count: loop_count,
                    span,
                    insight: format!(
                        "Agent is struggling with {}. Consider reverting or creating a reproduction script.",
                        tool_name
                    ),
                });
                i = j;
            }
        }
        i += 1;
    }
}

fn detect_apologies(events: &[AgentEvent], warnings: &mut Vec<PatternWarning>) {
    let apology_patterns = ["i apologize", "my mistake", "sorry", "i was wrong"];
    let mut apology_count = 0;

    for event in events {
        if let EventPayload::Message(msg) = &event.payload {
            let text_lower = msg.text.to_lowercase();
            for pattern in &apology_patterns {
                if text_lower.contains(pattern) {
                    apology_count += 1;
                    break;
                }
            }
        }
    }

    if apology_count > 3 {
        warnings.push(PatternWarning {
            pattern: "Excessive Apologies".to_string(),
            count: apology_count,
            span: "Throughout session".to_string(),
            insight: "Agent is frequently apologizing, indicating uncertainty or repeated errors."
                .to_string(),
        });
    }
}

fn detect_lazy_tools(events: &[AgentEvent], warnings: &mut Vec<PatternWarning>) {
    let mut lazy_count = 0;

    for i in 0..events.len().saturating_sub(1) {
        // Check for error result followed by tool call
        if let EventPayload::ToolResult(result) = &events[i].payload {
            if result.is_error {
                // Check if next event is a tool call
                if matches!(events[i + 1].payload, EventPayload::ToolCall(_)) {
                    // Check if there's reasoning between error and next call
                    let has_reasoning = events[i + 1..]
                        .iter()
                        .take(5)
                        .any(|e| matches!(e.payload, EventPayload::Reasoning(_)));

                    if !has_reasoning {
                        lazy_count += 1;
                    }
                }
            }
        }
    }

    if lazy_count > 2 {
        warnings.push(PatternWarning {
            pattern: "Lazy Tool Usage".to_string(),
            count: lazy_count,
            span: "Throughout session".to_string(),
            insight: "Agent is making tool calls without reasoning after errors.".to_string(),
        });
    }
}

fn detect_zombie_chains(events: &[AgentEvent], warnings: &mut Vec<PatternWarning>) {
    let mut chain_length = 0;
    let mut max_chain = 0;
    let mut last_user_msg_idx = None;

    for (i, event) in events.iter().enumerate() {
        match &event.payload {
            EventPayload::User(_) => {
                if chain_length > max_chain {
                    max_chain = chain_length;
                }
                chain_length = 0;
                last_user_msg_idx = Some(i);
            }
            EventPayload::ToolCall(_) => {
                chain_length += 1;
            }
            _ => {}
        }
    }

    if chain_length > max_chain {
        max_chain = chain_length;
    }

    if max_chain > 20 {
        let span = if let Some(idx) = last_user_msg_idx {
            if let (Some(start), Some(end)) = (events.get(idx), events.last()) {
                format_time_span(start.timestamp, end.timestamp)
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        };

        warnings.push(PatternWarning {
            pattern: "Zombie Chain".to_string(),
            count: max_chain,
            span,
            insight: format!(
                "Agent made {} tool calls without user interaction. Consider breaking down the task.",
                max_chain
            ),
        });
    }
}

fn detect_lint_ping_pong(events: &[AgentEvent], warnings: &mut Vec<PatternWarning>) {
    let mut edit_lint_cycles = 0;
    let mut i = 0;

    while i < events.len() {
        if let EventPayload::ToolCall(call) = &events[i].payload {
            if call.name == "Edit" || call.name == "Write" {
                let mut j = i + 1;
                while j < events.len() {
                    if let EventPayload::ToolCall(next_call) = &events[j].payload {
                        let tool_lower = next_call.name.to_lowercase();
                        if tool_lower.contains("lint") || tool_lower.contains("check") {
                            // Find result for this lint check
                            if let Some(result_event) = events[j + 1..].iter().find(|e| {
                                if let EventPayload::ToolResult(r) = &e.payload {
                                    r.tool_call_id == events[j].id
                                } else {
                                    false
                                }
                            }) {
                                if let EventPayload::ToolResult(result) = &result_event.payload {
                                    if result.is_error {
                                        edit_lint_cycles += 1;
                                    }
                                }
                            }
                        }
                    }
                    j += 1;
                    if j >= i + 10 {
                        break;
                    }
                }
            }
        }
        i += 1;
    }

    if edit_lint_cycles > 3 {
        warnings.push(PatternWarning {
            pattern: "Lint Ping-Pong".to_string(),
            count: edit_lint_cycles,
            span: "Throughout session".to_string(),
            insight: "Agent is oscillating between editing and fixing lint errors.".to_string(),
        });
    }
}

fn analyze_tool_usage(events: &[AgentEvent], info_items: &mut Vec<PatternInfo>) {
    let mut tool_counts: HashMap<String, usize> = HashMap::new();

    for event in events {
        if let EventPayload::ToolCall(call) = &event.payload {
            *tool_counts.entry(call.name.clone()).or_insert(0) += 1;
        }
    }

    let mut details = Vec::new();
    for (tool, count) in tool_counts.iter() {
        details.push(format!("{}: {} times", tool, count));
    }

    if !details.is_empty() {
        info_items.push(PatternInfo {
            category: "Tool Usage".to_string(),
            details,
        });
    }
}

fn calculate_score(warnings: &[PatternWarning]) -> u32 {
    let base_score: u32 = 100;
    let penalty_per_warning: u32 = 5;
    let total_penalty = warnings.len() as u32 * penalty_per_warning;
    base_score.saturating_sub(total_penalty)
}

fn format_time_span(start: DateTime<Utc>, end: DateTime<Utc>) -> String {
    let duration = end.signed_duration_since(start);
    let minutes = duration.num_minutes();
    let seconds = duration.num_seconds() % 60;
    format!("+{}m {:02}s", minutes, seconds)
}
