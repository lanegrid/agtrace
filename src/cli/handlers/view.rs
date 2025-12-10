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

    if raw {
        for log_file in &log_files {
            let content = fs::read_to_string(&log_file.path)
                .with_context(|| format!("Failed to read file: {}", log_file.path))?;
            println!("{}", content);
        }
        return Ok(());
    }

    let mut all_events = Vec::new();

    for log_file in &log_files {
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
            println!("  tool: {}", tool_name);
        }

        println!();
    }
}
