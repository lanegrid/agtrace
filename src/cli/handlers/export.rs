use crate::cli::output::{write_csv, write_jsonl};
use crate::model::EventType;
use crate::storage::Storage;
use crate::utils::resolve_effective_project_hash;
use anyhow::Result;
use std::path::PathBuf;
use std::str::FromStr;

pub fn handle(
    storage: &Storage,
    project_hash: Option<String>,
    session_id: Option<String>,
    event_type: Option<String>,
    all_projects: bool,
    out: PathBuf,
    format: String,
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
