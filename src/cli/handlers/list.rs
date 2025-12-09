use crate::cli::output::print_sessions_table;
use crate::model::Source;
use crate::storage::Storage;
use crate::utils::resolve_effective_project_hash;
use anyhow::Result;
use std::str::FromStr;

pub fn handle(
    storage: &Storage,
    project_hash: Option<String>,
    source: Option<String>,
    limit: usize,
    all_projects: bool,
    format: &str,
) -> Result<()> {
    let (effective_hash_string, all_projects) =
        resolve_effective_project_hash(project_hash.as_deref(), all_projects)?;
    let effective_project_hash = effective_hash_string.as_deref();

    let source_enum = source.as_deref().and_then(|s| Source::from_str(s).ok());
    let sessions = storage.list_sessions(
        effective_project_hash,
        source_enum,
        Some(limit),
        all_projects,
    )?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&sessions)?);
    } else {
        print_sessions_table(&sessions);
    }

    Ok(())
}
