use crate::cli::OutputFormat;
use crate::error::{Error, Result};
use crate::model::{Agent, Event};
use crate::storage;

use super::formatters::{format_duration, format_number, format_project_short};

pub fn cmd_find(
    id: &str,
    show_events: bool,
    events_limit: Option<usize>,
    format: OutputFormat,
    use_color: bool,
) -> Result<()> {
    // Try exact match first
    let execution = match storage::find_execution(id) {
        Ok(exec) => exec,
        Err(_) => {
            // If exact match fails, try prefix match
            let all_executions = storage::list_all_executions()?;
            let matches: Vec<_> = all_executions
                .into_iter()
                .filter(|e| e.id.starts_with(id))
                .collect();

            match matches.len() {
                0 => return Err(Error::ExecutionNotFound(id.to_string())),
                1 => matches.into_iter().next().unwrap(),
                _ => {
                    let mut error_msg = format!(
                        "Multiple executions match '{}'. Please provide more characters:\n",
                        id
                    );
                    for exec in matches.iter().take(10) {
                        error_msg.push_str(&format!("  {}\n", exec.id));
                    }
                    return Err(Error::Parse(error_msg));
                }
            }
        }
    };

    if format.is_json() {
        match format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&execution)?);
            }
            OutputFormat::Jsonl => {
                println!("{}", serde_json::to_string(&execution)?);
            }
            _ => unreachable!(),
        }
        return Ok(());
    }

    // Print compact summary format
    use nu_ansi_term::Color;

    println!();
    if use_color {
        println!(
            "{}",
            Color::Cyan.bold().paint(format!("Session: {}", execution.id))
        );
    } else {
        println!("Session: {}", execution.id);
    }
    println!();

    // Agent and project info
    let agent_name = match &execution.agent {
        Agent::ClaudeCode { model, .. } => format!("Claude Code ({})", model),
        Agent::Codex { model } => format!("Codex ({})", model),
    };

    let project = format_project_short(&execution.project_path);
    let branch_info = execution
        .git_branch
        .as_ref()
        .map(|b| format!(" ({})", b))
        .unwrap_or_default();

    println!("Agent:    {}", agent_name);
    println!("Project:  {}{}", project, branch_info);

    // Duration info
    if let (Some(duration), Some(ended)) = (execution.metrics.duration_seconds, execution.ended_at)
    {
        let duration_str = if duration < 60 {
            format_duration(duration)
        } else {
            let minutes = duration / 60;
            format!("{} minutes", minutes)
        };
        let start_time = execution.started_at.format("%b %d, %H:%M");
        let end_time = ended.format("%H:%M");
        println!("Duration: {} ({} - {})", duration_str, start_time, end_time);
    } else {
        println!("Started:  {}", execution.started_at.format("%b %d, %H:%M"));
    }
    println!();

    // Summary (if available)
    if !execution.summaries.is_empty() {
        println!("Summary:");
        for summary in &execution.summaries {
            println!("  {}", summary);
        }
        println!();
    }

    // Activity summary
    println!("Activity:");
    println!(
        "  User messages:     {}",
        execution.metrics.user_message_count
    );

    // Tool usage breakdown (compact)
    if !execution.metrics.tool_calls_by_name.is_empty() {
        let mut tools: Vec<_> = execution.metrics.tool_calls_by_name.iter().collect();
        tools.sort_by(|a, b| b.1.cmp(a.1));
        let tool_summary: Vec<String> = tools
            .iter()
            .take(5)
            .map(|(name, count)| format!("{}: {}", name, count))
            .collect();
        println!(
            "  Tool calls:        {} ({})",
            execution.metrics.tool_call_count,
            tool_summary.join(", ")
        );
    } else {
        println!("  Tool calls:        {}", execution.metrics.tool_call_count);
    }

    // Token usage
    println!(
        "  Tokens:            {} in / {} out",
        format_number(execution.metrics.input_tokens),
        format_number(execution.metrics.output_tokens)
    );

    if execution.metrics.cache_read_tokens > 0 {
        println!(
            "                     {} cache read",
            format_number(execution.metrics.cache_read_tokens)
        );
    }

    println!();

    // Event timeline hint
    if show_events {
        println!("Events ({}):", execution.events.len());
        println!();

        // Use provided limit or default to 20
        let event_limit = events_limit.unwrap_or(20);
        let events_to_show = execution.events.len().min(event_limit);

        for (i, event) in execution.events.iter().take(events_to_show).enumerate() {
            print_event(i, event, use_color);
        }

        if execution.events.len() > event_limit {
            println!();
            println!(
                "... and {} more events",
                execution.events.len() - event_limit
            );
            println!("Use --format json to see full event timeline, or --events-limit N to show more");
        }
    } else {
        println!("Use --events to see event timeline.");
    }

    Ok(())
}

pub fn print_event(index: usize, event: &Event, use_color: bool) {
    use nu_ansi_term::Color;

    match event {
        Event::UserMessage { timestamp, .. } => {
            let msg = format!("  [{}] {} - User message", index, timestamp);
            if use_color {
                println!("{}", Color::Blue.paint(msg));
            } else {
                println!("{}", msg);
            }
        }
        Event::AssistantMessage { timestamp, .. } => {
            let msg = format!("  [{}] {} - Assistant message", index, timestamp);
            if use_color {
                println!("{}", Color::Green.paint(msg));
            } else {
                println!("{}", msg);
            }
        }
        Event::Thinking {
            duration_ms,
            timestamp,
            ..
        } => {
            let duration = duration_ms
                .map(|ms| format!(" ({}ms)", ms))
                .unwrap_or_default();
            let msg = format!("  [{}] {} - Thinking{}", index, timestamp, duration);
            if use_color {
                println!("{}", Color::Magenta.paint(msg));
            } else {
                println!("{}", msg);
            }
        }
        Event::ToolCall {
            name, timestamp, ..
        } => {
            let msg = format!("  [{}] {} - Tool call: {}", index, timestamp, name);
            if use_color {
                println!("{}", Color::Cyan.paint(msg));
            } else {
                println!("{}", msg);
            }
        }
        Event::ToolResult { timestamp, .. } => {
            println!("  [{}] {} - Tool result", index, timestamp);
        }
        Event::FileSnapshot { timestamp, .. } => {
            println!("  [{}] {} - File snapshot", index, timestamp);
        }
    }
}
