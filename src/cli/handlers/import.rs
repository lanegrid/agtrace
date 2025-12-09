use crate::cli::import::{count_unique_sessions, import_vendor_logs};
use crate::cli::output::write_jsonl;
use crate::storage::Storage;
use anyhow::Result;
use std::path::PathBuf;

pub fn handle(
    storage: &Storage,
    source: String,
    root: Option<PathBuf>,
    project_root: Option<String>,
    session_id_prefix: Option<String>,
    all_projects: bool,
    dry_run: bool,
    out_jsonl: Option<PathBuf>,
) -> Result<()> {
    let events = import_vendor_logs(
        &source,
        root.as_ref(),
        project_root.as_deref(),
        session_id_prefix.as_deref(),
        all_projects,
    )?;

    if dry_run {
        println!(
            "Dry run: Would import {} events from {} sessions",
            events.len(),
            count_unique_sessions(&events)
        );
    } else {
        storage.save_events(&events)?;
        println!(
            "Imported {} events from {} sessions",
            events.len(),
            count_unique_sessions(&events)
        );
    }

    if let Some(out_path) = out_jsonl {
        write_jsonl(&out_path, &events)?;
        println!("Wrote events to {}", out_path.display());
    }

    Ok(())
}
