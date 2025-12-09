use crate::cli::output::print_stats;
use crate::model::Source;
use crate::storage::Storage;
use crate::utils::resolve_effective_project_hash;
use anyhow::Result;
use std::str::FromStr;

pub fn handle(
    storage: &Storage,
    project_hash: Option<String>,
    source: Option<String>,
    all_projects: bool,
) -> Result<()> {
    let (effective_hash_string, all_projects) =
        resolve_effective_project_hash(project_hash.as_deref(), all_projects)?;
    let effective_project_hash = effective_hash_string.as_deref();

    let source_enum = source.as_deref().and_then(|s| Source::from_str(s).ok());
    let sessions =
        storage.list_sessions(effective_project_hash, source_enum, None, all_projects)?;

    print_stats(&sessions);

    Ok(())
}
