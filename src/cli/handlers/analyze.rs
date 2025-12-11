use crate::db::Database;
use crate::model::{AgentEventV1, EventType};
use crate::providers::{ClaudeProvider, CodexProvider, GeminiProvider, ImportContext, LogProvider};
use anyhow::Result;
use chrono::DateTime;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct AnalysisReport {
    session_id: String,
    score: u32,
    warnings: Vec<PatternWarning>,
    info: Vec<PatternInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PatternWarning {
    pattern: String,
    count: usize,
    span: String,
    insight: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PatternInfo {
    category: String,
    details: Vec<String>,
}

pub fn handle(db: &Database, session_id: String, detect: String, format: String) -> Result<()> {
    let resolved_id = match db.find_session_by_prefix(&session_id)? {
        Some(full_id) => full_id,
        None => {
            let files = db.get_session_files(&session_id)?;
            if files.is_empty() {
                anyhow::bail!("Session not found: {}", session_id);
            }
            session_id.clone()
        }
    };

    let log_files = db.get_session_files(&resolved_id)?;

    if log_files.is_empty() {
        anyhow::bail!("Session not found: {}", session_id);
    }

    let main_files: Vec<_> = log_files
        .into_iter()
        .filter(|f| f.role != "sidechain")
        .collect();

    if main_files.is_empty() {
        anyhow::bail!("No main log files found for session: {}", session_id);
    }

    let mut all_events = Vec::new();

    for log_file in &main_files {
        let path = Path::new(&log_file.path);
        let provider: Box<dyn LogProvider> = if log_file.path.contains(".claude/") {
            Box::new(ClaudeProvider::new())
        } else if log_file.path.contains(".codex/") {
            Box::new(CodexProvider::new())
        } else if log_file.path.contains(".gemini/") {
            Box::new(GeminiProvider::new())
        } else {
            eprintln!("Warning: Unknown provider for file: {}", log_file.path);
            continue;
        };

        let context = ImportContext {
            project_root_override: None,
            session_id_prefix: None,
            all_projects: false,
        };

        match provider.normalize_file(path, &context) {
            Ok(mut events) => {
                all_events.append(&mut events);
            }
            Err(e) => {
                eprintln!("Warning: Failed to normalize {}: {}", log_file.path, e);
            }
        }
    }

    all_events.sort_by(|a, b| a.ts.cmp(&b.ts));

    let detectors: Vec<&str> = if detect == "all" {
        vec!["loops", "apology", "lazy_tool", "zombie", "lint_ping_pong"]
    } else {
        detect.split(',').collect()
    };

    let mut warnings = Vec::new();
    let mut info_items = Vec::new();

    for detector in detectors {
        match detector {
            "loops" => detect_loops(&all_events, &mut warnings),
            "apology" => detect_apologies(&all_events, &mut warnings),
            "lazy_tool" => detect_lazy_tools(&all_events, &mut warnings),
            "zombie" => detect_zombie_chains(&all_events, &mut warnings),
            "lint_ping_pong" => detect_lint_ping_pong(&all_events, &mut warnings),
            _ => eprintln!("Warning: Unknown detector: {}", detector),
        }
    }

    analyze_tool_usage(&all_events, &mut info_items);

    let score = calculate_score(&warnings);

    let report = AnalysisReport {
        session_id: resolved_id.clone(),
        score,
        warnings,
        info: info_items,
    };

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&report);
    }

    Ok(())
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
    let mut tool_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut tool_durations: std::collections::HashMap<String, Vec<u64>> =
        std::collections::HashMap::new();

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

fn print_report(report: &AnalysisReport) {
    println!(
        "Analysis Report for Session: {}",
        report.session_id.bright_blue()
    );

    let score_colored = if report.score >= 90 {
        format!("{}", report.score.to_string().green())
    } else if report.score >= 70 {
        format!("{}", report.score.to_string().yellow())
    } else {
        format!("{}", report.score.to_string().red())
    };

    let warning_text = if report.warnings.len() == 1 {
        "1 Warning"
    } else {
        &format!("{} Warnings", report.warnings.len())
    };

    println!("Score: {}/100 ({})", score_colored, warning_text);
    println!();

    for warning in &report.warnings {
        println!(
            "{} {} (Count: {})",
            "[WARN]".yellow(),
            warning.pattern.bold(),
            warning.count
        );
        println!("  Span: {}", warning.span);
        println!("  Insight: {}", warning.insight);
        println!();
    }

    for info in &report.info {
        println!("{} {}", "[INFO]".cyan(), info.category.bold());
        for detail in &info.details {
            println!("  - {}", detail);
        }
        println!();
    }
}
