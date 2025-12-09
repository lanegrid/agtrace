use crate::cli::output::print_events_timeline;
use crate::model::EventType;
use crate::storage::Storage;
use anyhow::Result;

pub fn handle(
    storage: &Storage,
    session_id: String,
    no_reasoning: bool,
    no_tool: bool,
    limit: Option<usize>,
    format: &str,
) -> Result<()> {
    let mut events = storage.load_session_events(&session_id)?;

    if no_reasoning {
        events.retain(|e| e.event_type != EventType::Reasoning);
    }

    if no_tool {
        events.retain(|e| {
            e.event_type != EventType::ToolCall && e.event_type != EventType::ToolResult
        });
    }

    if let Some(lim) = limit {
        events.truncate(lim);
    }

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&events)?);
    } else {
        print_events_timeline(&events);
    }

    Ok(())
}
