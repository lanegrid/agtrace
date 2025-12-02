use crate::cli::OutputFormat;
use crate::error::Result;
use crate::model::{Event, Execution, ToolStatus};
use crate::storage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::formatters::{format_date_short, format_duration, format_path_compact};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TraceToolUse {
    name: String,
    input: serde_json::Value,
    output: Option<String>,
    exit_code: Option<i32>,
    is_error: Option<bool>,
    status: ToolStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw_metadata: Option<serde_json::Value>,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum TraceStep {
    #[serde(rename = "thinking")]
    Thinking {
        content: String,
        timestamp: DateTime<Utc>,
    },
    #[serde(rename = "tool_use")]
    ToolUse { tool: TraceToolUse },
    #[serde(rename = "assistant_message")]
    Assistant {
        content: String,
        timestamp: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TraceTurn {
    user_message: Option<String>,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    steps: Vec<TraceStep>,
}

pub fn cmd_trace(
    agent: &str,
    id: &str,
    custom_path: Option<PathBuf>,
    turn_filter: Option<usize>,
    tools_only: bool,
    no_thinking: bool,
    format: OutputFormat,
    max_assistant_len: usize,
    max_output_len: usize,
    max_thinking_len: usize,
    use_color: bool,
) -> Result<()> {
    let execution = storage::find_execution_by_agent(agent, id, custom_path)?;

    let mut turns = build_turns(&execution);

    // Clamp assistant / thinking lengths for JSON/table views
    for turn in &mut turns {
        if max_assistant_len > 0 {
            for step in &mut turn.steps {
                if let TraceStep::Assistant { content, .. } = step {
                    *content = truncate(content, max_assistant_len);
                }
            }
        }
        if max_thinking_len > 0 {
            for step in &mut turn.steps {
                if let TraceStep::Thinking { content, .. } = step {
                    *content = truncate(content, max_thinking_len);
                }
            }
        }
    }

    if max_output_len > 0 {
        clamp_tool_outputs(&mut turns, max_output_len);
    }

    if format.is_json() {
        match format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&turns)?);
            }
            OutputFormat::Jsonl => {
                for turn in &turns {
                    println!("{}", serde_json::to_string(turn)?);
                }
            }
            _ => unreachable!(),
        }
        return Ok(());
    }

    print_trace_table(
        &execution,
        &turns,
        turn_filter,
        tools_only,
        no_thinking,
        max_assistant_len,
        max_output_len,
        max_thinking_len,
        use_color,
    );
    Ok(())
}

fn clamp_tool_outputs(turns: &mut [TraceTurn], max_output_len: usize) {
    for turn in turns {
        for step in &mut turn.steps {
            if let TraceStep::ToolUse { tool } = step {
                if let Some(out) = &mut tool.output {
                    *out = truncate(out, max_output_len);
                }
            }
        }
    }
}

fn build_turns(execution: &Execution) -> Vec<TraceTurn> {
    let mut turns = Vec::new();
    let mut current: Option<TraceTurn> = None;
    // Map call_id -> index into current.steps where a TraceStep::ToolUse resides
    let mut pending_calls: HashMap<Option<String>, usize> = HashMap::new();

    for event in &execution.events {
        match event {
            Event::UserMessage { content, timestamp } => {
                if let Some(turn) = current.take() {
                    turns.push(turn);
                }
                current = Some(TraceTurn {
                    user_message: Some(content.clone()),
                    started_at: *timestamp,
                    ended_at: Some(*timestamp),
                    steps: Vec::new(),
                });
                pending_calls.clear();
            }
            Event::Thinking {
                content, timestamp, ..
            } => {
                let ts = *timestamp;
                let turn = current.get_or_insert_with(|| TraceTurn {
                    user_message: None,
                    started_at: ts,
                    ended_at: Some(ts),
                    steps: Vec::new(),
                });
                turn.steps.push(TraceStep::Thinking {
                    content: content.clone(),
                    timestamp: ts,
                });
                turn.ended_at = Some(ts);
            }
            Event::ToolCall {
                name,
                input,
                call_id,
                timestamp,
            } => {
                let ts = *timestamp;
                let turn = current.get_or_insert_with(|| TraceTurn {
                    user_message: None,
                    started_at: ts,
                    ended_at: Some(ts),
                    steps: Vec::new(),
                });

                let tool_use = TraceToolUse {
                    name: name.clone(),
                    input: input.clone(),
                    output: None,
                    exit_code: None,
                    is_error: None,
                    status: ToolStatus::Unknown,
                    raw_metadata: None,
                    started_at: ts,
                    ended_at: None,
                };

                let idx = turn.steps.len();
                turn.steps.push(TraceStep::ToolUse { tool: tool_use });
                if call_id.is_some() {
                    pending_calls.insert(call_id.clone(), idx);
                }

                turn.ended_at = Some(ts);
            }
            Event::ToolResult {
                call_id,
                output,
                exit_code,
                is_error,
                status,
                raw_metadata,
                timestamp,
                ..
            } => {
                let ts = *timestamp;
                let turn = current.get_or_insert_with(|| TraceTurn {
                    user_message: None,
                    started_at: ts,
                    ended_at: Some(ts),
                    steps: Vec::new(),
                });

                if let Some(idx) = pending_calls
                    .get(call_id)
                    .copied()
                    .or_else(|| pending_calls.get(&None).copied())
                {
                    if let Some(TraceStep::ToolUse { tool }) = turn.steps.get_mut(idx) {
                        tool.output = Some(output.clone());
                        tool.exit_code = *exit_code;
                        tool.is_error = *is_error;
                        tool.status = *status;
                        tool.raw_metadata = raw_metadata.clone();
                        tool.ended_at = Some(ts);
                    }
                    pending_calls.remove(call_id);
                } else if let Some(TraceStep::ToolUse { tool }) = turn.steps.last_mut() {
                    if tool.output.is_none() {
                        tool.output = Some(output.clone());
                        tool.exit_code = *exit_code;
                        tool.is_error = *is_error;
                        tool.status = *status;
                        tool.raw_metadata = raw_metadata.clone();
                        tool.ended_at = Some(ts);
                    }
                } else {
                    // No existing ToolCall; create a synthetic ToolUse
                    turn.steps.push(TraceStep::ToolUse {
                        tool: TraceToolUse {
                            name: "unknown".to_string(),
                            input: serde_json::Value::Null,
                            output: Some(output.clone()),
                            exit_code: *exit_code,
                            is_error: *is_error,
                            status: *status,
                            raw_metadata: raw_metadata.clone(),
                            started_at: ts,
                            ended_at: Some(ts),
                        },
                    });
                }

                turn.ended_at = Some(ts);
            }
            Event::AssistantMessage { content, timestamp } => {
                let ts = *timestamp;
                let turn = current.get_or_insert_with(|| TraceTurn {
                    user_message: None,
                    started_at: ts,
                    ended_at: Some(ts),
                    steps: Vec::new(),
                });
                turn.steps.push(TraceStep::Assistant {
                    content: content.clone(),
                    timestamp: ts,
                });
                turn.ended_at = Some(ts);
            }
            Event::FileSnapshot { timestamp, .. } => {
                let ts = *timestamp;
                if let Some(turn) = current.as_mut() {
                    turn.ended_at = Some(ts);
                }
            }
        }
    }

    if let Some(turn) = current {
        turns.push(turn);
    }

    turns
}

fn print_trace_table(
    execution: &Execution,
    turns: &[TraceTurn],
    turn_filter: Option<usize>,
    tools_only: bool,
    no_thinking: bool,
    max_assistant_len: usize,
    max_output_len: usize,
    max_thinking_len: usize,
    use_color: bool,
) {
    use nu_ansi_term::Color;

    println!();
    let header = format!("Trace for session {} ({} turns)", execution.id, turns.len());
    if use_color {
        println!("{}", Color::Cyan.bold().paint(header));
    } else {
        println!("{}", header);
    }

    let working_dir = format_path_compact(&execution.working_dir, 80);
    let date = format_date_short(&execution.started_at);
    println!("Path:  {}", working_dir);
    println!("Date:  {}", date);
    println!();

    let indices: Vec<(usize, &TraceTurn)> = turns
        .iter()
        .enumerate()
        .filter(|(idx, _)| {
            if let Some(tn) = turn_filter {
                *idx + 1 == tn
            } else {
                true
            }
        })
        .collect();

    for (idx, turn) in &indices {
        let turn_no = idx + 1;
        let duration_str = turn
            .ended_at
            .map(|end| {
                let secs = end
                    .signed_duration_since(turn.started_at)
                    .num_seconds()
                    .max(0) as u64;
                format_duration(secs)
            })
            .unwrap_or_else(|| "-".to_string());

        let title = if let Some(msg) = &turn.user_message {
            let snippet = truncate(&single_line(msg), 80);
            format!("Turn {} ({}): {}", turn_no, duration_str, snippet)
        } else {
            format!("Turn {} ({}): <no user message>", turn_no, duration_str)
        };

        if use_color {
            println!("{}", Color::Yellow.bold().paint(title));
        } else {
            println!("{}", title);
        }

        // User message section
        if let Some(msg) = &turn.user_message {
            let label = if use_color {
                Color::Cyan.bold().paint("User").to_string()
            } else {
                "User".to_string()
            };
            println!("  {}: {}", label, truncate(&single_line(msg), 100));
        }

        // Mixed timeline section
        println!(
            "  {}:",
            if use_color {
                Color::Yellow.bold().paint("Timeline").to_string()
            } else {
                "Timeline".to_string()
            }
        );

        // For per-step durations, track previous timestamp within the turn
        let mut prev_ts = turn.started_at;

        for step in &turn.steps {
            match step {
                TraceStep::Thinking { content, timestamp } if !no_thinking && !tools_only => {
                    let secs = timestamp
                        .signed_duration_since(prev_ts)
                        .num_seconds()
                        .max(0) as u64;
                    let duration_str = if secs == 0 {
                        "0s".to_string()
                    } else {
                        format!("+{}", format_duration(secs))
                    };
                    let label = if use_color {
                        Color::Blue.bold().paint("thinking").to_string()
                    } else {
                        "thinking".to_string()
                    };
                    let limit = if max_thinking_len > 0 {
                        max_thinking_len
                    } else {
                        120
                    };
                    println!(
                        "    [{}] {}: {}",
                        duration_str,
                        label,
                        truncate(&single_line(content), limit)
                    );

                    prev_ts = *timestamp;
                }
                TraceStep::ToolUse { tool } => {
                    // Duration for this tool call = end - start (or 0s if unknown)
                    let secs = tool
                        .ended_at
                        .unwrap_or(tool.started_at)
                        .signed_duration_since(tool.started_at)
                        .num_seconds()
                        .max(0) as u64;
                    let duration_str = format_duration(secs);
                    let kind_label = if use_color {
                        Color::Green.bold().paint("tool").to_string()
                    } else {
                        "tool".to_string()
                    };
                    let header_text = format!(
                        "[{}] {} {} ({})",
                        duration_str, kind_label, tool.name, tool.status
                    );
                    if use_color {
                        let styled = match tool.status {
                            ToolStatus::Failure => Color::Red.bold().paint(header_text),
                            ToolStatus::Success => Color::Green.bold().paint(header_text),
                            ToolStatus::Unknown => Color::Yellow.bold().paint(header_text),
                        };
                        println!("    {}", styled);
                    } else {
                        println!("    {}", header_text);
                    }

                    if !tool.input.is_null() {
                        if let Some(obj) = tool.input.as_object() {
                            let mut parts = Vec::new();
                            for (k, v) in obj {
                                let vs = single_line(&v.to_string());
                                parts.push(format!("{}={}", k, vs));
                            }
                            let args = parts.join(", ");
                            println!("      args: {}", truncate(&args, 200));
                        } else {
                            let v = single_line(&tool.input.to_string());
                            println!("      args: {}", truncate(&v, 200));
                        }
                    }

                    if let Some(code) = tool.exit_code {
                        let code_str = code.to_string();
                        if use_color && tool.status == ToolStatus::Failure {
                            println!(
                                "      exit: {}",
                                Color::Red.bold().paint(code_str).to_string()
                            );
                        } else {
                            println!("      exit: {}", code_str);
                        }
                    }

                    if let Some(err) = tool.is_error {
                        println!("      is_error: {}", err);
                    }

                    if let Some(output) = &tool.output {
                        println!(
                            "      out: {}",
                            truncate(
                                &single_line(output),
                                if max_output_len > 0 {
                                    max_output_len
                                } else {
                                    60
                                }
                            )
                        );
                    }

                    if let Some(meta) = &tool.raw_metadata {
                        println!(
                            "      meta: {}",
                            truncate(&single_line(&meta.to_string()), 200)
                        );
                    }

                    // Advance prev_ts to the end of this tool call (or start if unknown)
                    prev_ts = tool.ended_at.unwrap_or(tool.started_at);
                }
                TraceStep::Assistant { content, timestamp } if !tools_only => {
                    let secs = timestamp
                        .signed_duration_since(prev_ts)
                        .num_seconds()
                        .max(0) as u64;
                    let duration_str = if secs == 0 {
                        "0s".to_string()
                    } else {
                        format!("+{}", format_duration(secs))
                    };
                    let label = if use_color {
                        Color::Purple.bold().paint("assistant").to_string()
                    } else {
                        "assistant".to_string()
                    };
                    let limit = if max_assistant_len > 0 {
                        max_assistant_len
                    } else {
                        100
                    };
                    println!(
                        "    [{}] {}: {}",
                        duration_str,
                        label,
                        truncate(&single_line(content), limit)
                    );

                    prev_ts = *timestamp;
                }
                _ => {}
            }
        }

        println!();
    }

    if indices.is_empty() {
        println!("(no turns matched the given filter)");
    }
}

fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }

    let mut result = String::new();
    let mut count = 0usize;
    let limit = max.saturating_sub(3); // reserve space for "..."

    for ch in s.chars() {
        if count >= limit {
            result.push_str("...");
            return result;
        }
        result.push(ch);
        count += 1;
    }

    // s is shorter than or equal to max chars
    result
}

fn single_line(s: &str) -> String {
    let replaced = s.replace(['\n', '\r'], " ");
    let mut out = String::new();
    let mut prev_space = false;
    for ch in replaced.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            prev_space = false;
            out.push(ch);
        }
    }
    out.trim().to_string()
}
