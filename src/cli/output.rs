use crate::model::*;
use anyhow::Result;
use std::path::PathBuf;

pub fn print_events_timeline(events: &[AgentEventV1]) {
    for event in events {
        let event_type_str = match event.event_type {
            EventType::UserMessage => "user_message",
            EventType::AssistantMessage => "assistant_message",
            EventType::SystemMessage => "system_message",
            EventType::Reasoning => "reasoning",
            EventType::ToolCall => "tool_call",
            EventType::ToolResult => "tool_result",
            EventType::FileSnapshot => "file_snapshot",
            EventType::SessionSummary => "session_summary",
            EventType::Meta => "meta",
            EventType::Log => "log",
        };
        let role_str = event
            .role
            .map(|r| format!("{:?}", r))
            .unwrap_or_else(|| "".to_string());

        println!("[{}] {:<20} (role={})", event.ts, event_type_str, role_str);

        if let Some(text) = &event.text {
            let preview = if text.chars().count() > 100 {
                let truncated: String = text.chars().take(97).collect();
                format!("{}...", truncated)
            } else {
                text.clone()
            };
            println!("  {}", preview);
        }

        if let Some(tool_name) = &event.tool_name {
            println!("  tool: {}", tool_name);
        }

        println!();
    }
}

pub fn print_stats(sessions: &[SessionSummary]) {
    let total_sessions = sessions.len();
    let total_events: usize = sessions.iter().map(|s| s.event_count).sum();
    let total_user_msgs: usize = sessions.iter().map(|s| s.user_message_count).sum();
    let total_tokens_in: u64 = sessions.iter().map(|s| s.tokens_input_total).sum();
    let total_tokens_out: u64 = sessions.iter().map(|s| s.tokens_output_total).sum();

    println!("OVERALL STATISTICS");
    println!("{}", "=".repeat(60));
    println!("Total Sessions:       {}", total_sessions);
    println!("Total Events:         {}", total_events);
    println!("Total User Messages:  {}", total_user_msgs);
    println!("Total Tokens Input:   {}", format_number(total_tokens_in));
    println!("Total Tokens Output:  {}", format_number(total_tokens_out));
    println!(
        "Total Tokens:         {}",
        format_number(total_tokens_in + total_tokens_out)
    );
}

pub fn write_jsonl(path: &PathBuf, events: &[AgentEventV1]) -> Result<()> {
    let mut lines = Vec::new();
    for event in events {
        lines.push(serde_json::to_string(event)?);
    }
    std::fs::write(path, lines.join("\n") + "\n")?;
    Ok(())
}

pub fn write_csv(path: &PathBuf, events: &[AgentEventV1]) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;

    wtr.write_record([
        "ts",
        "source",
        "session_id",
        "event_id",
        "event_type",
        "role",
        "text",
        "tool_name",
        "tokens_input",
        "tokens_output",
    ])?;

    for event in events {
        wtr.write_record([
            &event.ts,
            &format!("{:?}", event.source),
            event.session_id.as_deref().unwrap_or(""),
            event.event_id.as_deref().unwrap_or(""),
            &format!("{:?}", event.event_type),
            event
                .role
                .as_ref()
                .map(|r| format!("{:?}", r))
                .as_deref()
                .unwrap_or(""),
            event.text.as_deref().unwrap_or(""),
            event.tool_name.as_deref().unwrap_or(""),
            event
                .tokens_input
                .map(|t| t.to_string())
                .as_deref()
                .unwrap_or(""),
            event
                .tokens_output
                .map(|t| t.to_string())
                .as_deref()
                .unwrap_or(""),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push('_');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}
