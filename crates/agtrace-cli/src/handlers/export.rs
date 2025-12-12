use agtrace_index::Database;
use agtrace_types::{AgentEventV1, EventType};
use agtrace_providers::{ClaudeProvider, CodexProvider, GeminiProvider, ImportContext, LogProvider};
use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn handle(
    db: &Database,
    session_id: String,
    output: Option<PathBuf>,
    format: String,
    strategy: String,
) -> Result<()> {
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

    let processed_events = match strategy.as_str() {
        "clean" => apply_clean_strategy(&all_events),
        "reasoning" => apply_reasoning_strategy(&all_events),
        "raw" => all_events.clone(),
        _ => anyhow::bail!("Unknown strategy: {}", strategy),
    };

    let output_path = output.unwrap_or_else(|| {
        PathBuf::from(format!(
            "session_{}.{}",
            &resolved_id[..8],
            if format == "jsonl" { "jsonl" } else { "txt" }
        ))
    });

    match format.as_str() {
        "jsonl" => write_jsonl(&output_path, &processed_events)?,
        "text" => write_text(&output_path, &processed_events)?,
        _ => anyhow::bail!("Unsupported format: {}", format),
    }

    println!(
        "Exported {} events to {}",
        processed_events.len(),
        output_path.display()
    );

    Ok(())
}

fn apply_clean_strategy(events: &[AgentEventV1]) -> Vec<AgentEventV1> {
    let mut cleaned = Vec::new();
    let mut skip_until_next_success = false;

    for event in events.iter() {
        match event.event_type {
            EventType::ToolResult => {
                if let Some(exit_code) = event.tool_exit_code {
                    if exit_code != 0 {
                        skip_until_next_success = true;
                        continue;
                    } else {
                        skip_until_next_success = false;
                    }
                }
            }
            EventType::AssistantMessage => {
                if let Some(text) = &event.text {
                    let text_lower = text.to_lowercase();
                    if text_lower.contains("i apologize")
                        || text_lower.contains("my mistake")
                        || text_lower.contains("sorry")
                    {
                        continue;
                    }
                }
            }
            _ => {}
        }

        if !skip_until_next_success {
            let mut cleaned_event = event.clone();

            if let Some(text) = &cleaned_event.text {
                if text.len() > 5000 {
                    cleaned_event.text = Some(format!(
                        "{}...<truncated_output_for_training>",
                        text.chars().take(1000).collect::<String>()
                    ));
                }
            }

            cleaned.push(cleaned_event);
        }
    }

    cleaned
}

fn apply_reasoning_strategy(events: &[AgentEventV1]) -> Vec<AgentEventV1> {
    let mut reasoning_pairs = Vec::new();

    for i in 0..events.len() {
        if matches!(events[i].event_type, EventType::Reasoning) {
            reasoning_pairs.push(events[i].clone());

            if let Some(next) = events.get(i + 1) {
                if matches!(next.event_type, EventType::ToolCall) {
                    reasoning_pairs.push(next.clone());
                }
            }
        }
    }

    reasoning_pairs
}

fn write_jsonl(path: &Path, events: &[AgentEventV1]) -> Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to create file: {}", path.display()))?;

    for event in events {
        let json = serde_json::to_string(event)?;
        writeln!(file, "{}", json)?;
    }

    Ok(())
}

fn write_text(path: &Path, events: &[AgentEventV1]) -> Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to create file: {}", path.display()))?;

    for event in events {
        writeln!(file, "[{}] {:?}", event.ts, event.event_type)?;

        if let Some(text) = &event.text {
            writeln!(file, "{}", text)?;
        }

        if let Some(tool_name) = &event.tool_name {
            writeln!(file, "Tool: {}", tool_name)?;
        }

        writeln!(file)?;
    }

    Ok(())
}
