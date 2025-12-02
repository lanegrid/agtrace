use crate::cli::OutputFormat;
use crate::error::Result;
use crate::model::Agent;
use crate::storage;
use std::path::PathBuf;

use super::formatters::{format_duration, format_number, format_path};

pub fn cmd_show(
    agent: &str,
    id: &str,
    custom_path: Option<PathBuf>,
    show_events: bool,
    events_limit: Option<usize>,
    format: OutputFormat,
    use_color: bool,
) -> Result<()> {
    let execution = storage::find_execution_by_agent(agent, id, custom_path)?;

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
            Color::Cyan
                .bold()
                .paint(format!("Session: {}", execution.id))
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

    let working_dir = format_path(&execution.working_dir);
    let branch_info = execution
        .git_branch
        .as_ref()
        .map(|b| format!(" ({})", b))
        .unwrap_or_default();

    println!("Agent:    {}", agent_name);
    println!("Path:     {}{}", working_dir, branch_info);

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

        // Categorize tools into read (exploration) vs write (execution)
        let read_tools = ["Read", "Glob", "Grep", "WebFetch"];
        let write_tools = ["Write", "Edit", "Bash", "NotebookEdit"];

        let read_count: u64 = execution
            .metrics
            .tool_calls_by_name
            .iter()
            .filter(|(name, _)| read_tools.contains(&name.as_str()))
            .map(|(_, count)| *count as u64)
            .sum();

        let write_count: u64 = execution
            .metrics
            .tool_calls_by_name
            .iter()
            .filter(|(name, _)| write_tools.contains(&name.as_str()))
            .map(|(_, count)| *count as u64)
            .sum();

        if read_count > 0 || write_count > 0 {
            let total = read_count + write_count;
            let read_pct = (read_count as f64 / total as f64 * 100.0) as u32;
            println!(
                "  Read/Write ratio:  {}% / {}% (explore vs execute)",
                read_pct,
                100 - read_pct
            );
        }
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
            super::find::print_event(i, event, use_color);
        }

        if execution.events.len() > event_limit {
            println!();
            println!(
                "... and {} more events",
                execution.events.len() - event_limit
            );
            println!(
                "Use --format json to see full event timeline, or --events-limit N to show more"
            );
        }
    } else {
        println!("Use --events to see event timeline.");
    }

    Ok(())
}
