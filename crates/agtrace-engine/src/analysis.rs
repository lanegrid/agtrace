use agtrace_types::{AgentEventV1, EventType};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

impl Detector {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "loops" => Some(Detector::Loops),
            "apology" => Some(Detector::Apology),
            "lazy_tool" => Some(Detector::LazyTool),
            "zombie" => Some(Detector::Zombie),
            "lint_ping_pong" => Some(Detector::LintPingPong),
            _ => None,
        }
    }

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
    events: &[AgentEventV1],
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

fn detect_loops(events: &[AgentEventV1], warnings: &mut Vec<PatternWarning>) {
    let mut i = 0;
    while i < events.len() {
        if matches!(events[i].event_type, EventType::ToolCall) {
            let tool_name = events[i].tool_name.as_ref();
            let file_path = events[i].file_path.as_ref();

            let mut loop_count = 0;
            let mut j = i;
            let start_ts = &events[i].ts;
            let mut end_ts = start_ts;

            while j < events.len().saturating_sub(1) {
                if matches!(events[j].event_type, EventType::ToolCall)
                    && events[j].tool_name.as_ref() == tool_name
                    && events[j].file_path.as_ref() == file_path
                {
                    if let Some(result_idx) = events[j + 1..]
                        .iter()
                        .position(|e| matches!(e.event_type, EventType::ToolResult))
                    {
                        let result = &events[j + 1 + result_idx];
                        if result.tool_exit_code.unwrap_or(0) != 0 {
                            loop_count += 1;
                            end_ts = &result.ts;
                        } else {
                            break;
                        }
                    }
                }
                j += 1;
            }

            if loop_count >= 2 {
                let span = format_time_span(start_ts, end_ts);
                let pattern_desc = if let (Some(tool), Some(file)) = (tool_name, file_path) {
                    let filename = std::path::Path::new(file)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(file);
                    format!("{}({})", tool, filename)
                } else if let Some(tool) = tool_name {
                    tool.clone()
                } else {
                    "Unknown tool".to_string()
                };

                warnings.push(PatternWarning {
                    pattern: "Loop Detected".to_string(),
                    count: loop_count,
                    span,
                    insight: format!(
                        "Agent is struggling with {}. Consider reverting or creating a reproduction script.",
                        pattern_desc
                    ),
                });
                i = j;
            }
        }
        i += 1;
    }
}

fn detect_apologies(events: &[AgentEventV1], warnings: &mut Vec<PatternWarning>) {
    let apology_patterns = ["i apologize", "my mistake", "sorry", "i was wrong"];
    let mut apology_count = 0;

    for event in events {
        if matches!(event.event_type, EventType::AssistantMessage) {
            if let Some(text) = &event.text {
                let text_lower = text.to_lowercase();
                for pattern in &apology_patterns {
                    if text_lower.contains(pattern) {
                        apology_count += 1;
                        break;
                    }
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

fn detect_lazy_tools(events: &[AgentEventV1], warnings: &mut Vec<PatternWarning>) {
    let mut lazy_count = 0;

    for i in 0..events.len().saturating_sub(1) {
        if matches!(events[i].event_type, EventType::ToolResult)
            && events[i].tool_exit_code.unwrap_or(0) != 0
            && matches!(events[i + 1].event_type, EventType::ToolCall)
        {
            let has_reasoning = events[i + 1..]
                .iter()
                .take(5)
                .any(|e| matches!(e.event_type, EventType::Reasoning));

            if !has_reasoning {
                lazy_count += 1;
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

fn detect_zombie_chains(events: &[AgentEventV1], warnings: &mut Vec<PatternWarning>) {
    let mut chain_length = 0;
    let mut max_chain = 0;
    let mut last_user_msg_idx = None;

    for (i, event) in events.iter().enumerate() {
        match event.event_type {
            EventType::UserMessage => {
                if chain_length > max_chain {
                    max_chain = chain_length;
                }
                chain_length = 0;
                last_user_msg_idx = Some(i);
            }
            EventType::ToolCall => {
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
                format_time_span(&start.ts, &end.ts)
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

fn detect_lint_ping_pong(events: &[AgentEventV1], warnings: &mut Vec<PatternWarning>) {
    let mut edit_lint_cycles = 0;
    let mut i = 0;

    while i < events.len() {
        if matches!(events[i].event_type, EventType::ToolCall) {
            if let Some(tool_name) = &events[i].tool_name {
                if tool_name == "Edit" || tool_name == "Write" {
                    let mut j = i + 1;
                    while j < events.len() {
                        if matches!(events[j].event_type, EventType::ToolCall) {
                            if let Some(next_tool) = &events[j].tool_name {
                                if next_tool.to_lowercase().contains("lint")
                                    || next_tool.to_lowercase().contains("check")
                                {
                                    if let Some(result_idx) = events[j + 1..]
                                        .iter()
                                        .position(|e| matches!(e.event_type, EventType::ToolResult))
                                    {
                                        if events[j + 1 + result_idx].tool_exit_code.unwrap_or(0)
                                            != 0
                                        {
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

fn analyze_tool_usage(events: &[AgentEventV1], info_items: &mut Vec<PatternInfo>) {
    let mut tool_counts: HashMap<String, usize> = HashMap::new();
    let mut tool_durations: HashMap<String, Vec<u64>> = HashMap::new();

    for event in events {
        if let (EventType::ToolCall, Some(tool_name)) = (event.event_type, &event.tool_name) {
            *tool_counts.entry(tool_name.clone()).or_insert(0) += 1;
            if let Some(latency) = event.tool_latency_ms {
                tool_durations
                    .entry(tool_name.clone())
                    .or_default()
                    .push(latency);
            }
        }
    }

    let mut details = Vec::new();
    for (tool, count) in tool_counts.iter() {
        let avg_duration = if let Some(durations) = tool_durations.get(tool) {
            if !durations.is_empty() {
                let sum: u64 = durations.iter().sum();
                sum / durations.len() as u64
            } else {
                0
            }
        } else {
            0
        };

        if avg_duration > 0 {
            details.push(format!(
                "{}: {} times (Avg {}ms)",
                tool, count, avg_duration
            ));
        } else {
            details.push(format!("{}: {} times", tool, count));
        }
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

fn format_time_span(start: &str, end: &str) -> String {
    if let (Ok(start_time), Ok(end_time)) = (
        DateTime::parse_from_rfc3339(start),
        DateTime::parse_from_rfc3339(end),
    ) {
        let duration = end_time.signed_duration_since(start_time);
        let minutes = duration.num_minutes();
        let seconds = duration.num_seconds() % 60;
        format!("+{}m {:02}s", minutes, seconds)
    } else {
        "Unknown".to_string()
    }
}
