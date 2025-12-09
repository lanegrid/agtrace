use crate::cli::output::{write_csv, write_jsonl};
use crate::model::EventType;
use crate::storage::Storage;
use crate::utils::discover_project_root;
use anyhow::Result;
use std::path::PathBuf;

fn parse_event_type(s: &str) -> Option<EventType> {
    match s {
        "user_message" => Some(EventType::UserMessage),
        "assistant_message" => Some(EventType::AssistantMessage),
        "reasoning" => Some(EventType::Reasoning),
        "tool_call" => Some(EventType::ToolCall),
        "tool_result" => Some(EventType::ToolResult),
        "meta" => Some(EventType::Meta),
        _ => None,
    }
}

pub fn handle(
    storage: &Storage,
    project_hash: Option<String>,
    session_id: Option<String>,
    event_type: Option<String>,
    all_projects: bool,
    out: PathBuf,
    format: String,
) -> Result<()> {
    let (effective_project_hash, all_projects) = if project_hash.is_some() {
        (project_hash.as_deref(), false)
    } else if all_projects {
        (None, true)
    } else {
        let project_root_path = discover_project_root(None)?;
        let current_project_hash =
            crate::utils::project_hash_from_root(&project_root_path.to_string_lossy());
        (Some(current_project_hash.leak() as &str), false)
    };

    let event_type_enum = event_type.as_deref().and_then(parse_event_type);
    let events = storage.find_events(
        session_id.as_deref(),
        effective_project_hash,
        None,
        event_type_enum,
        None,
        all_projects,
    )?;

    match format.as_str() {
        "jsonl" => write_jsonl(&out, &events)?,
        "csv" => write_csv(&out, &events)?,
        _ => anyhow::bail!("Unsupported format: {}", format),
    }

    println!("Exported {} events to {}", events.len(), out.display());

    Ok(())
}
