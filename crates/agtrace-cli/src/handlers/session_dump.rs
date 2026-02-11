use crate::args::{DumpFormat, ViewModeArgs};
use agtrace_sdk::Client;
use anyhow::{Context, Result};

pub fn handle(
    client: &Client,
    session_id: String,
    raw: bool,
    format: DumpFormat,
    _view_mode: &ViewModeArgs,
) -> Result<()> {
    let session_handle = client.sessions().get(&session_id)?;
    let session = session_handle
        .assemble()
        .with_context(|| format!("Failed to assemble session: {}", session_id))?;

    // Convert session to event stream using SDK dump function
    let events = agtrace_sdk::dump::session_to_event_stream(&session, raw)?;

    // Output based on format
    match format {
        DumpFormat::Jsonl => {
            // One event per line
            for event in events {
                println!("{}", serde_json::to_string(&event)?);
            }
        }
        DumpFormat::Json => {
            // JSON array
            println!("{}", serde_json::to_string_pretty(&events)?);
        }
    }

    Ok(())
}
