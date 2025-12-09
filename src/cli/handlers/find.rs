use crate::cli::output::print_events_timeline;
use crate::model::EventType;
use crate::storage::Storage;
use crate::utils::discover_project_root;
use anyhow::Result;

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
    session_id: Option<String>,
    project_hash: Option<String>,
    text: Option<String>,
    event_type: Option<String>,
    limit: usize,
    all_projects: bool,
    format: &str,
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
        text.as_deref(),
        event_type_enum,
        Some(limit),
        all_projects,
    )?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&events)?);
    } else {
        print_events_timeline(&events);
    }

    Ok(())
}
