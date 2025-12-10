use crate::db::Database;
use crate::model::AgentEventV1;
use crate::providers::{ClaudeProvider, CodexProvider, GeminiProvider, ImportContext, LogProvider};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn handle(
    db: &Database,
    session_id: String,
    raw: bool,
    json: bool,
    _timeline: bool,
) -> Result<()> {
    // Try to resolve session ID (supports prefix matching)
    let resolved_id = match db.find_session_by_prefix(&session_id)? {
        Some(full_id) => full_id,
        None => {
            // If prefix matching fails, try exact match
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

    // Filter out sidechain files (e.g., Claude's agent-*.jsonl)
    let main_files: Vec<_> = log_files
        .into_iter()
        .filter(|f| f.role != "sidechain")
        .collect();

    if main_files.is_empty() {
        anyhow::bail!("No main log files found for session: {}", session_id);
    }

    if raw {
        for log_file in &main_files {
            let content = fs::read_to_string(&log_file.path)
                .with_context(|| format!("Failed to read file: {}", log_file.path))?;
            println!("{}", content);
        }
        return Ok(());
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

    if json {
        println!("{}", serde_json::to_string_pretty(&all_events)?);
    } else {
        print_events_timeline(&all_events);
    }

    Ok(())
}

fn print_events_timeline(events: &[AgentEventV1]) {
    for event in events {
        let event_type_str = format!("{:?}", event.event_type);
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
            print!("  tool: {}", tool_name);
            if let Some(file_path) = &event.file_path {
                print!(" ({})", file_path);
            }
            if let Some(file_op) = &event.file_op {
                print!(" [{}]", file_op);
            }
            if let Some(exit_code) = event.tool_exit_code {
                print!(" exit={}", exit_code);
            }
            println!();
        }

        // Display token information for assistant messages
        if matches!(event.event_type, crate::model::EventType::AssistantMessage) {
            let mut token_parts = Vec::new();
            if let Some(input) = event.tokens_input {
                token_parts.push(format!("in:{}", input));
            }
            if let Some(output) = event.tokens_output {
                token_parts.push(format!("out:{}", output));
            }
            if let Some(cached) = event.tokens_cached {
                if cached > 0 {
                    token_parts.push(format!("cached:{}", cached));
                }
            }
            if let Some(thinking) = event.tokens_thinking {
                if thinking > 0 {
                    token_parts.push(format!("thinking:{}", thinking));
                }
            }
            if let Some(tool) = event.tokens_tool {
                if tool > 0 {
                    token_parts.push(format!("tool:{}", tool));
                }
            }
            if !token_parts.is_empty() {
                println!("  tokens: {}", token_parts.join(", "));
            }
        }

        println!();
    }

    // Print session summary
    print_session_summary(events);
}

fn print_session_summary(events: &[AgentEventV1]) {
    if events.is_empty() {
        return;
    }

    println!("---");
    println!("Session Summary:");

    // Count events by type
    let mut user_count = 0;
    let mut assistant_count = 0;
    let mut tool_call_count = 0;
    let mut reasoning_count = 0;
    let mut file_ops = std::collections::HashMap::new();

    // Calculate total tokens
    let mut total_input = 0u64;
    let mut total_output = 0u64;
    let mut total_cached = 0u64;
    let mut total_thinking = 0u64;

    for event in events {
        match event.event_type {
            crate::model::EventType::UserMessage => user_count += 1,
            crate::model::EventType::AssistantMessage => assistant_count += 1,
            crate::model::EventType::ToolCall => tool_call_count += 1,
            crate::model::EventType::Reasoning => reasoning_count += 1,
            _ => {}
        }

        if let Some(file_op) = &event.file_op {
            *file_ops.entry(file_op.clone()).or_insert(0) += 1;
        }

        if let Some(t) = event.tokens_input {
            total_input += t;
        }
        if let Some(t) = event.tokens_output {
            total_output += t;
        }
        if let Some(t) = event.tokens_cached {
            total_cached += t;
        }
        if let Some(t) = event.tokens_thinking {
            total_thinking += t;
        }
    }

    println!("  Events: {} total", events.len());
    println!("    User messages: {}", user_count);
    println!("    Assistant messages: {}", assistant_count);
    println!("    Tool calls: {}", tool_call_count);
    println!("    Reasoning blocks: {}", reasoning_count);

    if !file_ops.is_empty() {
        println!("  File operations:");
        for (op, count) in file_ops.iter() {
            println!("    {}: {}", op, count);
        }
    }

    let total_tokens = total_input + total_output;
    if total_tokens > 0 {
        println!("  Tokens: {} total", total_tokens);
        println!("    Input: {}", total_input);
        println!("    Output: {}", total_output);
        if total_cached > 0 {
            println!("    Cached: {}", total_cached);
        }
        if total_thinking > 0 {
            println!("    Thinking: {}", total_thinking);
        }
    }

    // Calculate duration
    if let (Some(first), Some(last)) = (events.first(), events.last()) {
        if let (Ok(start), Ok(end)) = (
            chrono::DateTime::parse_from_rfc3339(&first.ts),
            chrono::DateTime::parse_from_rfc3339(&last.ts),
        ) {
            let duration = end.signed_duration_since(start);
            let minutes = duration.num_minutes();
            let seconds = duration.num_seconds() % 60;
            println!("  Duration: {}m {}s", minutes, seconds);
        }
    }
}
