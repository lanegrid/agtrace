use crate::cli::output::print_events_timeline;
use crate::model::EventType;
use crate::storage::Storage;
use crate::utils::resolve_effective_project_hash;
use anyhow::Result;
use std::str::FromStr;

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
    let (effective_hash_string, all_projects) =
        resolve_effective_project_hash(project_hash.as_deref(), all_projects)?;
    let effective_project_hash = effective_hash_string.as_deref();

    let event_type_enum = event_type
        .as_deref()
        .and_then(|s| EventType::from_str(s).ok());
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
